extern crate std;

use soroban_sdk::testutils::Address as _;
use soroban_sdk::{self, Address, Env};
use stellar_scaffold_macro::import_asset;

import_asset!("native");
import_asset!("USDC:GC6RVI3M7DM5RFEZVBDLGOC4QJHEW66Q4TCTXGGLNCL7EXR5BM2VWJCG");

#[test]
pub fn test_macro_native_production() {
    let env = &Env::default();
    let symbol = native::token_client(env).try_symbol();
    assert_eq!(symbol.is_err(), true);
    native::register(env);

    let client = native::stellar_asset_client(env);

    assert_eq!(
        native::contract_id(env).to_string(),
        to_string(
            env,
            "CB56OQJZFJXSSKFK3MXJZ4TLJAJFWH6KXN6BAWHQSJDZPHZFVBJ353HU"
        )
    );
    assert_eq!(client.symbol(), to_string(env, "native"));

    // Check one more call is successful (does nothing)
    native::register(env);

    assert_eq!(native::to_min_unit(1.0f64), 10000000);
    assert_eq!(native::from_min_unit(10000000), 1.0f64);
}

#[test]
pub fn test_native_unit_test() {
    let env = &Env::default();
    let admin = &Address::generate(env);
    let sac = test_native::register(env, admin);
    let client = test_native::stellar_asset_client(env, &sac);

    assert_eq!(client.admin(), *admin);
    assert_eq!(client.balance(admin), 1000000000);

    assert_eq!(test_native::to_min_unit(1.0f64), 10000000);
    assert_eq!(native::from_min_unit(10000000), 1.0f64);
}

#[test]
pub fn test_macro_usdc_production() {
    let env = &Env::default();
    let symbol = USDC::token_client(env).try_symbol();
    assert_eq!(symbol.is_err(), true);
    USDC::register(env);

    let client = USDC::stellar_asset_client(env);

    assert_eq!(
        USDC::contract_id(env).to_string(),
        to_string(
            env,
            "CAW2XLHJ6X4N2L343AJPVWL6DOI444B6TOHVCGHCOS4GF3TERVGTCAM7"
        )
    );
    assert_eq!(client.symbol(), to_string(env, "USDC"));
    assert_eq!(
        client.admin().to_string(),
        to_string(
            env,
            "GC6RVI3M7DM5RFEZVBDLGOC4QJHEW66Q4TCTXGGLNCL7EXR5BM2VWJCG"
        )
    );
}

#[test]
pub fn test_usdc_unit_test() {
    let env = &Env::default();
    let admin = &Address::generate(env);
    let sac = test_USDC::register(env, admin);
    let client = test_USDC::stellar_asset_client(env, &sac);

    assert_eq!(client.admin(), *admin);
    assert_eq!(client.balance(admin), 1000000000);
}

pub fn to_string(env: &Env, s: &str) -> soroban_sdk::String {
    soroban_sdk::String::from_str(env, s)
}
