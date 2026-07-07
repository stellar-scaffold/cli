fn main() {
    crate_git_revision::init();

    emit_local_protocol_version();

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

/// Derive the Stellar protocol version the local network should run from the
/// `stellar-cli` pin, and expose it as the `LOCAL_PROTOCOL_VERSION` compile-time
/// env var. stellar-cli's major version tracks the protocol, so this stays correct
/// across upgrades without a hardcoded number — bump the dependency and the local
/// network follows.
///
/// In a normal workspace build the pin lives in the workspace `Cargo.toml` two
/// levels up. During `cargo package`/`publish` the crate is copied to
/// `target/package/<crate>/`, where that path no longer exists — but Cargo inlines
/// the resolved version into the crate's own manifest (`workspace = true` becomes a
/// concrete `version`). Read whichever manifest actually holds the pin so the
/// build script works in both contexts.
fn emit_local_protocol_version() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let crate_toml = std::path::Path::new(&manifest_dir).join("Cargo.toml");
    // crates/stellar-scaffold-cli -> workspace root is two levels up.
    let workspace_toml = std::path::Path::new(&manifest_dir).join("../../Cargo.toml");

    let version = read_stellar_cli_version(&workspace_toml)
        .or_else(|| read_stellar_cli_version(&crate_toml))
        .expect(
            "could not find a stellar-cli version pin in the workspace or crate Cargo.toml \
             for the protocol version",
        );

    // The pin is a semver requirement (e.g. "=27.0.0"); take the major from its
    // first comparator. stellar-cli's major version tracks the protocol.
    let req = semver::VersionReq::parse(&version)
        .unwrap_or_else(|e| panic!("invalid stellar-cli version requirement \"{version}\": {e}"));
    let major = req
        .comparators
        .first()
        .unwrap_or_else(|| {
            panic!("stellar-cli version requirement \"{version}\" has no comparator")
        })
        .major;
    println!("cargo:rustc-env=LOCAL_PROTOCOL_VERSION={major}");
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
