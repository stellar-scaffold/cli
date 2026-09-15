// This lets use reference types in the std library for testing
extern crate std;

use super::*;
use soroban_sdk::{
    Address, Env,
    testutils::{Address as _, EnvTestConfig},
    token::StellarAssetClient,
};

fn generate_client<'a>(env: &Env, admin: &Address) -> ContractClient<'a> {
    let contract_id = Address::generate(env);
    env.mock_all_auths();
    let contract_id = env.register_at(&contract_id, Contract, (admin,));
    env.set_auths(&[]); // clear auths
    ContractClient::new(env, &contract_id)
}

fn init_test<'a>(env: &'a Env) -> (Address, StellarAssetClient<'a>, ContractClient<'a>) {
    let admin = Address::generate(env);
    let client = generate_client(env, &admin);
    // This is needed because we want to call a function from within the context of the contract
    // In this case we want to get the address of the XLM contract registered by the constructor
    let sac_address = env.as_contract(&client.address, || xlm::contract_id(env));
    (admin, StellarAssetClient::new(env, &sac_address), client)
}

#[test]
fn constructed_correctly() {
    let env = &Env::default();
    let (admin, sac, client) = init_test(env);
    // Check that the admin is set correctly
    assert_eq!(sac.admin(), admin.clone());
    // Check that the contract has a balance of 1 XLM
    assert_eq!(sac.balance(&client.address), xlm::to_stroops(1));
}

#[test]
fn test_networks() {
    let env = Env::default();
    let ledger = env.ledger();
    todo!(
        "Change the network id in Env and test expected address are created in xlm::SERIALIZED_ASSET, etc"
    )
}
