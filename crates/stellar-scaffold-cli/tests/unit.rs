#![cfg(not(feature = "integration-tests"))]
use stellar_scaffold_test::{AssertExt, TestEnv};

#[test]
fn no_environments_toml_ends_after_contract_build() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        let stderr = env.scaffold("build").assert().success().stderr_as_str();
        assert!(
            stderr.contains("Finished"),
            "expected the 'Finished' message, got: {stderr}"
        );
    });
}

#[test]
fn uses_manifest_path_for_build_command() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        env.set_environments_toml(
            r#"
development.accounts = [
    { name = "alice" },
]

[development.network]
rpc-url = "http://localhost:8000/rpc"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
hello_world.client = false
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
        );

        let stderr = env
            .stellar_scaffold_custom_dir(
                "build",
                &["--manifest-path", "./soroban-init-boilerplate/Cargo.toml"],
                &env.cwd.join(".."),
            )
            .assert()
            .success()
            .stderr_as_str();

        assert!(stderr.contains("Build Complete"));
    });
}

#[test]
fn init_copies_frontend_template() {
    let env = TestEnv::new_empty();

    // Use a unique project name to avoid pre-existing directory issue
    let project_name = format!(
        "my-project-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let project_path = env.cwd.join(&project_name);
    if project_path.exists() {
        std::fs::remove_dir_all(&project_path).unwrap();
    }
    assert!(!project_path.exists());
    // `--template react` keeps this non-interactive: the framework is chosen and
    // the package manager defaults to npm, so no prompts in CI.
    env.scaffold("init")
        .args([project_path.to_str().unwrap(), "--template", "react"])
        .assert()
        .success();
    // The monorepo is instantiated to a single `app/`; verify the promoted
    // template and the rewritten workspace root.
    assert!(project_path.join("package.json").exists());
    assert!(project_path.join("app").exists());
    assert!(project_path.join("app/src").exists());
    assert!(project_path.join("app/tsconfig.json").exists());
    // The chosen package manager is recorded in scaffold.yml, not package.json.
    let config = std::fs::read_to_string(project_path.join("scaffold.yml")).unwrap();
    assert!(config.contains("\n  package-manager: npm@"));
}

#[test]
fn init_builds_with_the_projects_toolchain() {
    let env = TestEnv::new_empty();
    let project_path = env.cwd.join("toolchain-project");

    // rustup sets RUSTUP_TOOLCHAIN for the processes it starts, which would
    // override the project's rust-toolchain.toml. Without it, a build started
    // from outside the project uses rustup's default toolchain instead.
    env.scaffold("init")
        .args([project_path.to_str().unwrap(), "--template", "none"])
        .env_remove("RUSTUP_TOOLCHAIN")
        .assert()
        .success();

    let toolchain = std::fs::read_to_string(project_path.join("rust-toolchain.toml")).unwrap();
    let channel = toolchain
        .lines()
        .find_map(|line| line.strip_prefix("channel = "))
        .unwrap()
        .trim_matches('"');
    let wasms: Vec<_> = walkdir::WalkDir::new(project_path.join("target/stellar"))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "wasm"))
        .collect();
    assert!(!wasms.is_empty(), "init built no contracts");
    for wasm in wasms {
        // The Wasm's `rsver` metadata records the rustc that built it.
        let bytes = std::fs::read(wasm.path()).unwrap();
        assert!(
            bytes
                .windows(channel.len())
                .any(|w| w == channel.as_bytes()),
            "{} wasn't built with Rust {channel}",
            wasm.path().display()
        );
    }
}

/// A version 2 `scaffold.yml` for a project with no Cargo workspace: one
/// network, `testnet`, with the given extra settings, and the given
/// `contracts:` section (indented two spaces).
fn no_cargo_scaffold_yml(network: &str, contracts: &str) -> String {
    format!("version: 2\nnetworks:\n  testnet:\n{network}contracts:\n{contracts}")
}

/// A contract that is already deployed, with no client so the build makes no
/// network calls.
const DEPLOYED_CONTRACT: &str = "  xlm:
    type: contract
    source: CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
    client: false
    networks: { testnet: }
";

/// Run `build --network testnet` in `env`, which has no `Cargo.toml`.
fn build_without_cargo(env: &TestEnv, args: &[&str]) -> assert_cmd::Command {
    let mut cmd = env.stellar_scaffold_custom_dir("build", &["--network", "testnet"], &env.cwd);
    cmd.args(args);
    cmd
}

/// A TypeScript-only project, with no `Cargo.toml` and no `contracts/`, only
/// references deployed contracts, so it builds without cargo and without any
/// accounts to sign with.
#[test]
fn builds_without_cargo_when_no_contract_is_a_workspace_crate() {
    let env = TestEnv::new_empty();
    env.modify_file(
        "scaffold.yml",
        &no_cargo_scaffold_yml("", DEPLOYED_CONTRACT),
    );
    let stderr = build_without_cargo(&env, &[])
        .assert()
        .success()
        .stderr_as_str();
    assert!(stderr.contains("No Cargo workspace found"), "{stderr}");
    assert!(!stderr.contains("cargo metadata"), "{stderr}");
}

/// A `workspace` contract still needs cargo; the error names it.
#[test]
fn workspace_contracts_need_cargo() {
    let env = TestEnv::new_empty();
    env.modify_file(
        "scaffold.yml",
        &no_cargo_scaffold_yml(
            "    allow-deploy: true\n    accounts: [me]\n",
            "  hello:
    type: workspace
    source: hello-world
    networks: { testnet: }
",
        ),
    );
    let stderr = build_without_cargo(&env, &[])
        .assert()
        .failure()
        .stderr_as_str();
    assert!(
        stderr.contains("contract `hello` is `type: workspace`"),
        "{stderr}"
    );
    assert!(stderr.contains("Cargo.toml"), "{stderr}");
}

/// Without a version 2 `scaffold.yml`, or with `--compile-only`, a missing
/// Cargo workspace is still cargo's error.
#[test]
fn without_cargo_other_builds_report_cargos_error() {
    let env = TestEnv::new_empty();
    build_without_cargo(&env, &[])
        .assert()
        .failure()
        .stderr(predicates::str::contains("could not find `Cargo.toml`"));

    env.modify_file(
        "scaffold.yml",
        &no_cargo_scaffold_yml("", DEPLOYED_CONTRACT),
    );
    build_without_cargo(&env, &["--compile-only"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("could not find `Cargo.toml`"));
}
