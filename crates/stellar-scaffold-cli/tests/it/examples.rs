use rstest::rstest;
use stellar_scaffold_test::{AssertExt, TestEnv, rpc_url};

#[rstest]
#[case::hello_world("stellar/hello_world")]
#[case::account("stellar/account")]
#[case::alloc("stellar/alloc")]
#[case::atomic_swap("stellar/atomic_swap")]
#[case::auth("stellar/auth")]
#[case::bls_signature("stellar/bls_signature")]
#[case::custom_types("stellar/custom_types")]
#[case::deep_contract_auth("stellar/deep_contract_auth")]
#[case::errors("stellar/errors")]
#[case::eth_abi("stellar/eth_abi")]
#[case::events("stellar/events")]
#[case::fuzzing("stellar/fuzzing")]
#[case::import_ark_bn254("stellar/import_ark_bn254")]
#[case::increment("stellar/increment")]
#[case::increment_with_fuzz("stellar/increment_with_fuzz")]
#[case::increment_with_pause("stellar/increment_with_pause")]
#[case::liquidity_pool("stellar/liquidity_pool")]
#[case::logging("stellar/logging")]
#[case::mint_lock("stellar/mint-lock")]
#[case::other_custom_types("stellar/other_custom_types")]
#[case::pause("stellar/pause")]
#[case::simple_account("stellar/simple_account")]
#[case::single_offer("stellar/single_offer")]
#[case::timelock("stellar/timelock")]
#[case::token("stellar/token")]
#[case::ttl("stellar/ttl")]
fn test_adding_and_building_example_work(#[case] input: &str) {
    TestEnv::from("soroban-init-boilerplate", |env| {
        env.set_environments_toml(format!(
            r#"
[development]
network = {{ rpc-url = "{}", network-passphrase = "Standalone Network ; February 2017" }}

accounts = [
    "alice",
]
[development.contracts]
soroban_hello_world_contract.client = false
soroban_increment_contract.client = false
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
            rpc_url()
        ));

        println!("starting test in directory {:?}", env.cwd);
        eprintln!("test directory {:?}", env.cwd);

        assert!(
            env.cwd.join("contracts").is_dir(),
            "no contracts directory found"
        );
        fs_extra::dir::remove(env.cwd.join("contracts"))
            .expect("failed to remove contracts directory");

        let mut cmd = env.scaffold("generate");
        let mut cmd = cmd
            .arg("contract")
            .arg("--from")
            .arg(input)
            .arg("--output")
            .arg(format!("{}/contracts/example", env.cwd.display()));

        if let Ok(cache_dir) = &std::env::var("CI_CACHE") {
            std::fs::create_dir_all(cache_dir).unwrap();
            cmd = cmd.env("XDG_CACHE_HOME", cache_dir);
        }

        cmd.assert().success().stdout_as_str();

        // Run scaffold_build and assert success
        env.scaffold_build("development", true).assert().success();
    });
}
