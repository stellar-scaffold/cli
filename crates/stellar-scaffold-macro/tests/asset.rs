use sha2::{Digest, Sha256};
use soroban_sdk::{Address, Bytes, Env, testutils::Ledger};
use stellar_build::Network;
use stellar_scaffold_macro::import_asset;
use stellar_xdr::curr::{self as xdr, WriteXdr};

import_asset!("native");
import_asset!("xlm");
import_asset!("USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN");
import_asset!("LONGASSET123:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN");

const ISSUER: &str = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

fn env_for(passphrase: &str) -> Env {
    let env = Env::default();
    env.ledger()
        .set_network_id(Sha256::digest(passphrase).into());
    env
}

fn issued_asset(code: &str) -> xdr::Asset {
    let issuer = ISSUER.parse().unwrap();
    match code.parse().unwrap() {
        xdr::AssetCode::CreditAlphanum4(asset_code) => {
            xdr::Asset::CreditAlphanum4(xdr::AlphaNum4 { asset_code, issuer })
        }
        xdr::AssetCode::CreditAlphanum12(asset_code) => {
            xdr::Asset::CreditAlphanum12(xdr::AlphaNum12 { asset_code, issuer })
        }
    }
}

// Independent, off-chain reference for networks/assets without a known address.
fn expected_address(env: &Env, passphrase: &str, asset: xdr::Asset) -> Address {
    let preimage = xdr::HashIdPreimage::ContractId(xdr::HashIdPreimageContractId {
        network_id: xdr::Hash(Sha256::digest(passphrase).into()),
        contract_id_preimage: xdr::ContractIdPreimage::Asset(asset),
    });
    let hash = Sha256::digest(preimage.to_xdr(xdr::Limits::none()).unwrap());
    Address::from_str(env, &stellar_strkey::Contract(hash.into()).to_string())
}

#[test]
fn native_addresses_follow_the_runtime_network() {
    for (network, expected) in [
        (
            Network::Local,
            "CDMLFMKMMD7MWZP3FKUBZPVHTUEDLSX4BYGYKH4GCESXYHS3IHQ4EIG4",
        ),
        (
            Network::Testnet,
            "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
        ),
        (
            Network::Futurenet,
            "CB64D3G7SM2RTH6JSGG34DDTFTQ5CFDKVDZJZSODMCX4NJ2HV2KN7OHT",
        ),
        (
            Network::Mainnet,
            "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA",
        ),
    ] {
        let env = env_for(network.passphrase());
        let expected = Address::from_str(&env, expected);
        assert_eq!(native::contract_id(&env), expected);
        assert_eq!(xlm::contract_id(&env), expected);
        assert_eq!(native::token_client(&env).address, expected);
        assert_eq!(native::stellar_asset_client(&env).address, expected);
        assert_eq!(xlm::token_client(&env).address, expected);
        assert_eq!(xlm::stellar_asset_client(&env).address, expected);
    }
}

#[test]
fn issued_addresses_follow_the_runtime_network() {
    for (network, expected) in [
        (
            Network::Local,
            "CB5SYISL2JCNQQRPFS5H4EFEESWUSNTDYMUNQX7TWZE45MYWYEYWCHAU",
        ),
        (
            Network::Testnet,
            "CA2E53VHFZ6YSWQIEIPBXJQGT6VW3VKWWZO555XKRQXYJ63GEBJJGHY7",
        ),
        (
            Network::Futurenet,
            "CBYZIQLTWJKSC34FJSCOGEQ63BR4YQWAKUDZDBMKIPUBBEMPRUMB5Z24",
        ),
        (
            Network::Mainnet,
            "CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75",
        ),
    ] {
        let env = env_for(network.passphrase());
        let expected = Address::from_str(&env, expected);
        assert_eq!(USDC::contract_id(&env), expected);
        assert_eq!(USDC::token_client(&env).address, expected);
        assert_eq!(USDC::stellar_asset_client(&env).address, expected);

        let expected = expected_address(&env, network.passphrase(), issued_asset("LONGASSET123"));
        assert_eq!(LONGASSET123::contract_id(&env), expected);
        assert_eq!(LONGASSET123::token_client(&env).address, expected);
        assert_eq!(LONGASSET123::stellar_asset_client(&env).address, expected);
    }
}

#[test]
fn custom_networks_use_the_ledger_network_id() {
    let passphrase = "Custom asset runtime test network";
    let env = env_for(passphrase);
    let expected = expected_address(&env, passphrase, xdr::Asset::Native);
    assert_eq!(native::contract_id(&env), expected);
    assert_eq!(xlm::contract_id(&env), expected);
    assert_eq!(
        USDC::contract_id(&env),
        expected_address(&env, passphrase, issued_asset("USDC"))
    );
    assert_eq!(
        LONGASSET123::contract_id(&env),
        expected_address(&env, passphrase, issued_asset("LONGASSET123"))
    );
}

#[test]
fn resolving_does_not_deploy_and_clients_work_after_deployment() {
    let env = env_for(Network::Testnet.passphrase());
    let address = USDC::contract_id(&env);
    let token = USDC::token_client(&env);
    let admin = USDC::stellar_asset_client(&env);

    let asset = issued_asset("USDC").to_xdr(xdr::Limits::none()).unwrap();
    // Deployment would fail if resolving or constructing clients had already
    // deployed the SAC. No issuer account needs to exist to derive its address.
    assert_eq!(
        env.deployer()
            .with_stellar_asset(Bytes::from_slice(&env, &asset))
            .deploy(),
        address
    );

    assert_eq!(admin.admin(), Address::from_str(&env, ISSUER));
    assert_eq!(token.decimals(), 7);
}
