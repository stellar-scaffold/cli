//! `build --build-clients` with a version 2 `scaffold.yml`.

use stellar_scaffold_test::{TestEnv, rpc_url};

/// A version 2 `scaffold.yml` with one network, `test`, pointing at the test
/// RPC, and the given `contracts:` section (indented two spaces).
fn scaffold_yml(accounts: &str, contracts: &str) -> String {
    format!(
        r"version: 2
networks:
  test:
    rpc-url: {}
    network-passphrase: Standalone Network ; February 2017
    start-container: false
    accounts: [{accounts}]
contracts:
{contracts}",
        rpc_url()
    )
}

/// Run `build --build-clients --network test`.
fn build(env: &TestEnv) -> std::process::Output {
    env.scaffold_build("development", true)
        .args(["--network", "test"])
        .output()
        .expect("Failed to execute command")
}

fn index_ts(env: &TestEnv) -> String {
    std::fs::read_to_string(env.cwd.join("app-lib/clients/index.ts")).unwrap_or_default()
}

/// A contract that fails to deploy fails the build, after the others are
/// deployed and exported.
#[test]
fn failed_deploy_fails_the_build() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        env.modify_file(
            "scaffold.yml",
            &scaffold_yml(
                "me",
                r#"  hello:
    type: workspace
    source: soroban-hello-world-contract
    networks: { test: }
  token:
    type: workspace
    source: soroban-token-contract
    # The constructor panics when decimal > 18.
    args: { admin: "${account.me}", decimal: 19, name: Bad, symbol: BAD }
    networks: { test: }
"#,
            ),
        );
        let output = build(env);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{stderr}");
        assert!(
            stderr.contains("failed to deploy or generate clients for: token"),
            "{stderr}"
        );
        assert!(index_ts(env).contains("export const hello = "));
    });
}

/// A contract's `signer` signs its deploy and `after-deploy` calls, not the
/// network's default account.
#[test]
fn signer_signs_deploy_and_after_deploy() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        crate::util::fund(env, "bob");
        let before = crate::util::sequence(env, "bob");
        env.modify_file(
            "scaffold.yml",
            &scaffold_yml(
                "me, bob",
                "  counter:
    type: workspace
    source: soroban-increment-contract
    signer: bob
    after-deploy: [increment]
    networks: { test: }
",
            ),
        );
        let output = build(env);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        // me (the default) uploads the Wasm; bob deploys and calls increment.
        assert_eq!(crate::util::sequence(env, "bob") - before, 2);
    });
}

/// A client removed from scaffold.yml is no longer exported from index.ts,
/// even though its generated package is still on disk.
#[test]
fn removed_client_is_not_exported() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        let hello = "  hello:
    type: workspace
    source: soroban-hello-world-contract
    networks: { test: }
";
        let counter = "  counter:
    type: workspace
    source: soroban-increment-contract
    networks: { test: }
";
        env.modify_file(
            "scaffold.yml",
            &scaffold_yml("me", &format!("{hello}{counter}")),
        );
        assert!(build(env).status.success());
        assert!(index_ts(env).contains("export const counter = "));

        env.modify_file("scaffold.yml", &scaffold_yml("me", hello));
        assert!(build(env).status.success());
        assert!(env.cwd.join("app-lib/clients/counter").exists());
        let index = index_ts(env);
        assert!(index.contains("export const hello = "));
        assert!(!index.contains("export const counter = "), "{index}");
    });
}
