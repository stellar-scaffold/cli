#![no_std]
use soroban_sdk::{Address, Env, contract, contractimpl};

stellar_registry::import_asset!("xlm");

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Constructor to initialize the contract with an admin and a random number
    pub fn __constructor(env: &Env, admin: Address) {
        // Require auth from the admin to make the transfer
        admin.require_auth();
        // This is for testing purposes. Ensures that the XLM contract set up for unit testing and local network
        xlm::register(env, &admin);
        // Send the contract an amount of XLM to play with
        xlm::token_client(env).transfer(
            &admin,
            env.current_contract_address(),
            &xlm::to_stroops(1),
        );
    }
}

#[cfg(test)]
mod test;
