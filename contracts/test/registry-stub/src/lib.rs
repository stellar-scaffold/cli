#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
enum Key {
    Manager,
    Recorded,
}

#[contract]
pub struct RegistryStub;

#[contractimpl]
impl RegistryStub {
    pub fn __constructor(env: &Env, manager: &Address) {
        env.storage().instance().set(&Key::Manager, &manager);
    }

    /// Mirrors the real registry's manager-only entry points: requires the
    /// stored manager's auth and records the value so tests can assert the
    /// call went through.
    pub fn manager_only(env: &Env, value: u32) -> u32 {
        let manager: Address = env.storage().instance().get(&Key::Manager).unwrap();
        manager.require_auth();
        env.storage().instance().set(&Key::Recorded, &value);
        value
    }

    pub fn recorded(env: &Env) -> Option<u32> {
        env.storage().instance().get(&Key::Recorded)
    }
}
