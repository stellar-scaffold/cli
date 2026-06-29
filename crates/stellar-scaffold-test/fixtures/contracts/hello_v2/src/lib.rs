#![no_std]
use soroban_sdk::{Address, BytesN, Env, String, contract, contractimpl, symbol_short};

const ADMIN: soroban_sdk::Symbol = symbol_short!("ADMIN");

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    pub fn __constructor(env: &Env, admin: &Address) {
        env.storage().instance().set(&ADMIN, admin);
    }

    pub fn upgrade(env: &Env, new_wasm_hash: BytesN<32>) {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    pub fn hi(_env: &Env, to: String) -> String {
        to
    }
}
