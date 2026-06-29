use std::fs;
use stellar_cli::config::locator;
use stellar_scaffold_test::{AssertExt, TestEnv, rpc_url};
#[tokio::test]
async fn create_two_accounts() {
    TestEnv::from_async("soroban-init-boilerplate", async |env| {
        env.set_environments_toml(format!(
            r#"
[development]
network = {{ rpc-url = "{}", network-passphrase = "Standalone Network ; February 2017" }}

accounts = [
    "alice",
    {{ name = "bob" }},
]
[development.contracts]
soroban_hello_world_contract.client = false
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
            rpc_url(),
        ));
        for dir in fs::read_dir(&env.cwd).unwrap() {
            println!("Found directory: {dir:?}");
        }

        let stderr = env
            .scaffold_build("development", true)
            .assert()
            .success()
            .stderr_as_str();
        println!("{stderr}");
        assert!(stderr.contains("Creating keys for \"alice\""));
        assert!(stderr.contains("Creating keys for \"bob\""));

        // check that they dont get overwritten if build is run again
        let stderr = env
            .scaffold_build("development", true)
            .assert()
            .success()
            .stderr_as_str();
        assert!(stderr.contains("identity with the name \'alice\' already exists"));
        assert!(stderr.contains("identity with the name \'bob\' already exists"));
        for dir in fs::read_dir(env.cwd.join(".config/stellar")).unwrap() {
            println!("Found directory: {dir:?}");
        }

        // check that they're actually funded
        let cmd = stellar_cli::commands::keys::fund::Cmd {
            network: stellar_cli::config::network::Args {
                rpc_url: Some(rpc_url()),
                network_passphrase: Some("Standalone Network ; February 2017".to_string()),
                rpc_headers: vec![],
                network: None,
            },
            address: stellar_cli::commands::keys::public_key::Cmd {
                hd_path: None,
                locator: locator::Args {
                    config_dir: Some(env.config_dir()),
                },
                name: Some(stellar_cli::config::UnresolvedMuxedAccount::AliasOrSecret(
                    "alice".to_string(),
                )),
                ledger: false,
            },
        };
        cmd.run(&stellar_cli::commands::global::Args::default())
            .await
            .unwrap();
    })
    .await;
}

#[test]
fn funding_existing_account_toml() {
    TestEnv::from("soroban-init-boilerplate", |env| {
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
            rpc_url()
        ));

        // Create alice.toml manually, simulating a pre-existing identity
        env.stellar("keys")
            .args([
                "generate",
                "alice",
                "--network-passphrase",
                "\"Standalone Network ; February 2017\"",
                "--rpc-url",
                rpc_url().as_str(),
            ])
            .assert()
            .success();

        // Run scaffold_build and assert success
        env.scaffold_build("development", true).assert().success();
    });
}
