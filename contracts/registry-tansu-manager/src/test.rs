#![allow(clippy::needless_pass_by_value)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Bytes, Env};

use crate::{RegistryTansuManager, RegistryTansuManagerClient};

// Wasm-import the standalone tansu-stub so we can register it as the Tansu
// the manager queries during `__check_auth`.
mod tansu_stub_wasm {
    soroban_sdk_tools::contractimport!(file = "../../target/stellar/local/tansu_stub.wasm");
}

#[test]
fn constructor_stores_values() {
    let env = Env::default();
    let tansu = env.register(tansu_stub_wasm::WASM, ());
    let registry = Address::generate(&env);
    let project_key = Bytes::from_slice(&env, &[7u8; 16]);
    let manager = env.register(
        RegistryTansuManager,
        (tansu.clone(), project_key.clone(), registry.clone()),
    );
    let client = RegistryTansuManagerClient::new(&env, &manager);
    assert_eq!(client.tansu(), tansu);
    assert_eq!(client.project_key(), project_key);
    assert_eq!(client.registry(), registry);
}
