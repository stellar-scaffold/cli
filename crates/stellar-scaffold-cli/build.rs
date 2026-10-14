fn main() {
    crate_git_revision::init();

    emit_pinned_stellar_cli_version();

    // cargo_bin!("stellar-scaffold-reporter") in integration tests expands to
    // env!("CARGO_BIN_EXE_stellar-scaffold-reporter"), which Cargo sets for
    // same-package binaries and dev-dependency binaries during `cargo test` but
    // NOT during `cargo clippy --tests`.  Emitting it here ensures it is always
    // present at compile time regardless of how the crate is being built.
    let out_dir = std::env::var("OUT_DIR").unwrap();
    // OUT_DIR = target/<profile>/build/<hash>/out — 3 levels up is target/<profile>/
    let target_dir = std::path::Path::new(&out_dir).ancestors().nth(3).unwrap();
    let exe_suffix = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };
    println!(
        "cargo:rustc-env=CARGO_BIN_EXE_stellar-scaffold-reporter={}",
        target_dir
            .join(format!("stellar-scaffold-reporter{exe_suffix}"))
            .display()
    );
}

/// Expose the pinned `stellar-cli` version as `PINNED_STELLAR_CLI_VERSION` env
/// var so `doctor` can report the expected version alongside the one actually
/// on user's PATH. Read from workspace, falling back to crate manifest.
fn emit_pinned_stellar_cli_version() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let crate_toml = std::path::Path::new(&manifest_dir).join("Cargo.toml");
    let workspace_toml = std::path::Path::new(&manifest_dir).join("../../Cargo.toml");

    let version = read_stellar_cli_version(&workspace_toml)
        .or_else(|| read_stellar_cli_version(&crate_toml))
        .expect("could not find a stellar-cli version in workspace or crate Cargo.toml");
    println!("cargo:rustc-env=PINNED_STELLAR_CLI_VERSION={version}");
}

/// Read the `stellar-cli` version pin from a manifest, checking both the
/// `[workspace.dependencies]` table (workspace root) and the `[dependencies]`
/// table (a leaf crate, or a packaged crate with the pin inlined by Cargo).
/// Returns `None` if the file is absent or holds no such pin.
fn read_stellar_cli_version(path: &std::path::Path) -> Option<String> {
    println!("cargo:rerun-if-changed={}", path.display());
    let contents = std::fs::read_to_string(path).ok()?;
    let parsed: toml::Table = contents
        .parse()
        .unwrap_or_else(|e| panic!("{} is not valid TOML: {e}", path.display()));

    let deps = parsed
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .or_else(|| parsed.get("dependencies"))?;
    deps.get("stellar-cli")?
        .get("version")?
        .as_str()
        .map(str::to_owned)
}
