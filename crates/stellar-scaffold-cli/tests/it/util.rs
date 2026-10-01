//! Helpers shared across integration tests.

use stellar_scaffold_test::{TestEnv, rpc_url};

/// Create `account` in the test environment's keystore and fund it on the
/// test network.
pub fn fund(env: &TestEnv, account: &str) {
    let output = env
        .stellar("keys")
        .args([
            "generate",
            account,
            "--fund",
            "--rpc-url",
            &rpc_url(),
            "--network-passphrase",
            "Standalone Network ; February 2017",
        ])
        .output()
        .expect("stellar keys generate");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Sequence number of `account`'s ledger entry. Every transaction an account
/// submits increments it, so it shows who actually signed.
pub fn sequence(env: &TestEnv, account: &str) -> i64 {
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
