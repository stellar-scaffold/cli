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
        assert!(cmd.contains("Removed contract alias: soroban_hello_world_contract"));
        assert!(cmd.contains("Removed account: alice"));
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
        assert!(
            calling_clean_again
                .contains("Failed to remove contract alias soroban_hello_world_contract")
        ); // it has already been removed
        assert!(calling_clean_again.contains("Failed to remove account alice")); // it has already been removed
    })
    .await;
}

#[tokio::test]
async fn when_identity_is_used_in_multiple_envs() {
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


[staging]
network = {{ rpc-url = "{}", network-passphrase = "Standalone Network ; February 2017" }}

accounts = [
    "alice",
]
"#,
            rpc_url(),
            rpc_url(),
        ));

        env.scaffold_build("development", true)
           .assert()
           .success();

        let cmd = env.scaffold("clean").assert().success().stderr_as_str();
        assert!(cmd.contains("Skipping cleaning identity \"alice\". It is being used in other environments: [Staging]."));
    }).await;
}
