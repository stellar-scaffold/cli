use stellar_scaffold_test::{AssertExt, TestEnv, rpc_url};
#[tokio::test]
async fn test_clean() {
    TestEnv::from_async("soroban-init-boilerplate", async |env| {
        env.set_environments_toml(format!(
            r#"
[development]
network = {{ rpc-url = "{}", network-passphrase = "Standalone Network ; February 2017" }}

accounts = [
    "alice",
]
[development.contracts]
soroban_hello_world_contract.client = true
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
            rpc_url(),
        ));

        env.scaffold_build("development", true).assert().success();

        let cmd = env.scaffold("clean").assert().success().stderr_as_str();
        assert!(cmd.contains("Starting workspace cleanup"));
        assert!(cmd.contains("Removed app-lib/clients/soroban_hello_world_contract"));
        assert!(cmd.contains("Removed app-lib/clients/index.ts"));
        assert!(cmd.contains(
            "Removed contract alias: soroban-init-boilerplate-soroban_hello_world_contract"
        ));
        assert!(cmd.contains("Removed .scaffold/state.json"));
        // Keys are machine-wide, shared with other projects
        assert!(!cmd.contains("Removed account"));
        env.stellar("keys")
            .args(["address", "alice"])
            .assert()
            .success();
    })
    .await;
}

#[tokio::test]
async fn when_calling_clean_again() {
    TestEnv::from_async("soroban-init-boilerplate", async |env| {
        env.set_environments_toml(format!(
            r#"
[development]
network = {{ rpc-url = "{}", network-passphrase = "Standalone Network ; February 2017" }}

accounts = [
    "alice",
]
[development.contracts]
soroban_hello_world_contract.client = true
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
            rpc_url(),
        ));

        env.scaffold_build("development", true).assert().success();

        env.scaffold("clean").assert().success();

        let calling_clean_again = env.scaffold("clean").assert().success().stderr_as_str();
        assert!(calling_clean_again.contains("Starting workspace cleanup"));
        assert!(calling_clean_again.contains("Skipping target clean"));
        // Nothing left to remove, and nothing to complain about
        assert!(!calling_clean_again.contains("contract alias"));
        assert!(!calling_clean_again.contains("state.json"));
    })
    .await;
}
