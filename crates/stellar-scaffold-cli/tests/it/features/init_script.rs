use stellar_scaffold_test::{TestEnv, rpc_url};

#[test]
fn build_command_runs_init() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        env.set_environments_toml(format!(
            r#"
development.accounts = [
{{ name = "alice" }},
{{ name = "bob" }},
]

[development.network]
rpc-url = "{}"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_hello_world_contract.client = false
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false

[development.contracts.soroban_token_contract]
client = true
constructor_args = """
STELLAR_ACCOUNT=alice --symbol ABND --decimal 7 --name abundance --admin alice
"""
after_deploy = """
mint --amount 2000000 --to alice
"""
"#,
            rpc_url()
        ));

        let output = env
            .scaffold_build("development", true)
            .output()
            .expect("Failed to execute command");

        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        // ensure the invoke commands are run with the proper source account
        assert!(output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains(" -- mint --amount 2000000 --to alice")
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains(
            "✅ After deploy script for \"soroban_token_contract\" completed successfully"
        ));
        // ensure setting STELLAR_ACCOUNT works
        env.set_environments_toml(format!(
            r#"
development.accounts = [
{{ name = "alice" }},
{{ name = "bob" }},
]

[development.network]
rpc-url = "{}"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_hello_world_contract.client = false
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false

[development.contracts.soroban_token_contract]
client = true
constructor_args = """
STELLAR_ACCOUNT=bob --symbol ABND --decimal 7 --name abundance --admin bob 
"""
after_deploy = """
STELLAR_ACCOUNT=bob mint --amount 2000000 --to bob 
"""
"#,
            rpc_url()
        ));
        let output = env
            .scaffold_build("development", true)
            .output()
            .expect("Failed to execute command");

        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        // ensure the invoke commands are run with the proper source account
        assert!(output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("--source-account bob -- mint --amount 2000000 --to bob")
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains(
            "✅ After deploy script for \"soroban_token_contract\" completed successfully"
        ));
    });
}

#[test]
fn init_handles_quotations_and_subcommands_in_script() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        env.set_environments_toml(format!(
            r#"
    development.accounts = [
    {{ name = "me" }},
    ]

    [development.network]
    rpc-url = "{}"
    network-passphrase = "Standalone Network ; February 2017"

    [development.contracts]
    soroban_hello_world_contract.client = false
    soroban_increment_contract.client = false
    soroban_auth_contract.client = false
    soroban_token_contract.client = false

    [development.contracts.soroban_custom_types_contract]
    client = true
    after_deploy = """
    test_init --resolution 300000 --assets '[{{"Stellar": "$(stellar contract id asset --asset native)"}} ]' --decimals 14 --base '{{"Stellar":"$(stellar contract id asset --asset native)"}}'
    """
    "#, rpc_url()
        ));

        let output = env
            .scaffold_build("development", true)
            .output()
            .expect("Failed to execute command");

        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

        // Ensure the command executed successfully
        assert!(output.status.success());

        // Check for the presence of the after_deploy commands in the output
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(" -- test_init --resolution 300000")
        );

        // Check for successful after deploy message
        assert!(String::from_utf8_lossy(&output.stderr).contains(
            "✅ After deploy script for \"soroban_custom_types_contract\" completed successfully"
        ));
    });
}

#[test]
fn init_scripts_run_in_specified_order() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        // First configuration: custom_types then token
        env.set_environments_toml(format!(
            r#"
development.accounts = [
{{ name = "alice" }},
{{ name = "bob" }},
]

[development.network]
rpc-url = "{}"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_hello_world_contract.client = false
soroban_increment_contract.client = false
soroban_auth_contract.client = false

[development.contracts.soroban_custom_types_contract]
client = true
after_deploy = """
test_init --resolution 300000 --assets '[{{"Stellar": "$(stellar contract id asset --asset native)"}} ]' --decimals 14 --base '{{"Stellar":"$(stellar contract id asset --asset native)"}}'
"""

[development.contracts.soroban_token_contract]
client = true
constructor_args = """
STELLAR_ACCOUNT=bob --symbol ABND --decimal 7 --name abundance --admin bob 
"""
after_deploy = """
STELLAR_ACCOUNT=bob mint --amount 2000000 --to bob 
"""
"#, rpc_url()
        ));

        let output = env
            .scaffold_build("development", true)
            .output()
            .expect("Failed to execute command");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success());

        // Check order of initialization
        let custom_types_index = stderr
            .find("Running after_deploy script for \"soroban_custom")
            .expect("Custom types init not found");
        let token_index = stderr
            .find("Running after_deploy script for \"soroban_token")
            .expect("Token init not found");
        assert!(
            custom_types_index < token_index,
            "Custom types should be initialized before token"
        );

        // Second configuration: token then custom_types
        env.set_environments_toml(format!(
            r#"
development.accounts = [
{{ name = "alice" }},
{{ name = "bob" }},
]

[development.network]
rpc-url = "{}"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_hello_world_contract.client = false
soroban_increment_contract.client = false
soroban_auth_contract.client = false

[development.contracts.soroban_token_contract]
client = true
constructor_args = """
STELLAR_ACCOUNT=bob --symbol ABND --decimal 7 --name abundance --admin bob 
"""
after_deploy = """
STELLAR_ACCOUNT=bob mint --amount 2000000 --to bob 
"""

[development.contracts.soroban_custom_types_contract]
client = true
after_deploy = """
test_init --resolution 300000 --assets '[{{"Stellar": "$(stellar contract id asset --asset native)"}} ]' --decimals 14 --base '{{"Stellar":"$(stellar contract id asset --asset native)"}}'
"""
"#, rpc_url()));

        let output = env
            .scaffold_build("development", true)
            .output()
            .expect("Failed to execute command");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success());

        // Check order of initialization
        let token_index = stderr
            .find("Running after_deploy script for \"soroban_token")
            .expect("Token init not found");
        let custom_types_index = stderr
            .find("Running after_deploy script for \"soroban_custom")
            .expect("Custom types init not found");
        assert!(
            token_index < custom_types_index,
            "Token should be initialized before custom types"
        );
    });
}

/// Sequence number of `account`'s ledger entry. Every transaction an account
/// submits increments it, so it shows who actually signed.
fn sequence(env: &TestEnv, account: &str) -> i64 {
    let output = env
        .stellar("keys")
        .args(["address", account])
        .output()
        .expect("stellar keys address");
    let address = String::from_utf8(output.stdout).unwrap().trim().to_string();
    let client = soroban_rpc::Client::new(&rpc_url()).unwrap();
    let entry = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(client.get_account(&address))
        .expect("account exists");
    entry.seq_num.0
}

/// `STELLAR_ACCOUNT=` must decide who signs the deploy and after-deploy
/// transactions, not only what the printed command shows.
#[test]
fn stellar_account_prefix_signs_deploy_and_after_deploy() {
    TestEnv::from("soroban-init-boilerplate", |env| {
        // Create bob first so his sequence number can be compared.
        let funded = env
            .stellar("keys")
            .args([
                "generate",
                "bob",
                "--fund",
                "--rpc-url",
                &rpc_url(),
                "--network-passphrase",
                "Standalone Network ; February 2017",
            ])
            .output()
            .expect("stellar keys generate");
        assert!(
            funded.status.success(),
            "{}",
            String::from_utf8_lossy(&funded.stderr)
        );
        let before = sequence(env, "bob");

        env.set_environments_toml(format!(
            r#"
development.accounts = [
{{ name = "alice" }},
{{ name = "bob" }},
]

[development.network]
rpc-url = "{}"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_hello_world_contract.client = false
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false

[development.contracts.soroban_token_contract]
client = true
constructor_args = """
STELLAR_ACCOUNT=bob --symbol ABND --decimal 7 --name abundance --admin bob
"""
after_deploy = """
STELLAR_ACCOUNT=bob mint --amount 2000000 --to bob
"""
"#,
            rpc_url()
        ));
        let output = env
            .scaffold_build("development", true)
            .output()
            .expect("Failed to execute command");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        // alice (the default) uploads the Wasm; bob submits the deploy and the mint.
        assert_eq!(sequence(env, "bob") - before, 2);
    });
}
