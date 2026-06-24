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
/// `stellar-cli` pin in the workspace `Cargo.toml`, and expose it as the
/// `LOCAL_PROTOCOL_VERSION` compile-time env var. stellar-cli's major version
/// tracks the protocol, so this stays correct across upgrades without a hardcoded
/// number — bump the dependency and the local network follows.
fn emit_local_protocol_version() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    // crates/stellar-scaffold-cli -> workspace root is two levels up.
    let workspace_toml = std::path::Path::new(&manifest_dir).join("../../Cargo.toml");
    println!("cargo:rerun-if-changed={}", workspace_toml.display());

    let contents = std::fs::read_to_string(&workspace_toml)
        .expect("failed to read workspace Cargo.toml for protocol version");
    let parsed: toml::Table = contents
        .parse()
        .expect("workspace Cargo.toml is not valid TOML");

    let version = parsed["workspace"]["dependencies"]["stellar-cli"]["version"]
        .as_str()
        .expect("stellar-cli workspace dependency must pin a version string");
    // Strip any semver-requirement prefix (e.g. "=27.0.0") before taking the major.
    let major = version
        .trim_start_matches(['=', '^', '~', '>', '<', ' '])
        .split('.')
        .next()
        .expect("stellar-cli version is empty");
    assert!(
        major.chars().all(|c| c.is_ascii_digit()),
        "could not parse a protocol major version from stellar-cli = \"{version}\""
    );
    println!("cargo:rustc-env=LOCAL_PROTOCOL_VERSION={major}");
}
