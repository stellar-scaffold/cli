use proc_macro2::TokenStream;
use stellar_xdr::curr as xdr;
use xdr::WriteXdr;

use quote::{format_ident, quote};

pub fn parse_asset(str: &str) -> Result<(xdr::Asset, String), xdr::Error> {
    if str == "native" || str == "xlm" {
        return Ok((xdr::Asset::Native, str.to_string()));
    }
    let split: Vec<&str> = str.splitn(2, ':').collect();
    assert!(split.len() == 2, "invalid asset \"{str}\"");
    let code = split[0];
    let issuer: xdr::AccountId = split[1].parse()?;
    let re = regex::Regex::new("^[[:alnum:]]{1,12}$").expect("regex failed");
    assert!(re.is_match(code), "invalid asset \"{str}\"");
    let asset_code: xdr::AssetCode = code.parse()?;
    Ok((
        match asset_code {
            xdr::AssetCode::CreditAlphanum4(asset_code) => {
                xdr::Asset::CreditAlphanum4(xdr::AlphaNum4 { asset_code, issuer })
            }
            xdr::AssetCode::CreditAlphanum12(asset_code) => {
                xdr::Asset::CreditAlphanum12(xdr::AlphaNum12 { asset_code, issuer })
            }
        },
        code.to_string(),
    ))
}

/// Embed the asset's XDR identity and derive its contract address on the
/// executing network at runtime.
pub fn parse_literal(lit_str: &syn::LitStr) -> TokenStream {
    let (asset, code) = parse_asset(&lit_str.value()).unwrap();
    let asset_xdr = asset.to_xdr(xdr::Limits::none()).unwrap();
    let asset_xdr = syn::LitByteStr::new(&asset_xdr, lit_str.span());
    let mod_name = format_ident!("{code}");
    quote! {
        #[allow(non_snake_case)]
        pub(crate) mod #mod_name {
            #[allow(unused_imports)]
            use super::*;
            /// Derive the Stellar Asset Contract address for the executing network.
            /// This does not deploy the contract or check whether it exists.
            pub fn contract_id(env: &soroban_sdk::Env) -> soroban_sdk::Address {
                let asset = soroban_sdk::Bytes::from_slice(env, #asset_xdr);
                env.deployer().with_stellar_asset(asset).deployed_address()
            }
            /// Create a Stellar Asset Client for the asset which provides an admin interface
            pub fn stellar_asset_client<'a>(env: &soroban_sdk::Env) -> soroban_sdk::token::StellarAssetClient<'a> {
                soroban_sdk::token::StellarAssetClient::new(env, &contract_id(env))
            }
            /// Create a Token Client for the asset's standard token interface
            pub fn token_client<'a>(env: &soroban_sdk::Env) -> soroban_sdk::token::TokenClient<'a> {
                soroban_sdk::token::TokenClient::new(env, &contract_id(env))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const ISSUER: &str = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

    #[test]
    fn parse_native() {
        for name in ["native", "xlm"] {
            let (asset, code) = parse_asset(name).unwrap();
            assert_eq!(asset, xdr::Asset::Native);
            assert_eq!(code, name);
        }
    }

    #[test]
    fn parse_issued_assets() {
        for code in ["USDC", "LONGASSET123"] {
            let (asset, parsed_code) = parse_asset(&format!("{code}:{ISSUER}")).unwrap();
            assert_eq!(parsed_code, code);
            match asset {
                xdr::Asset::CreditAlphanum4(asset) => {
                    assert_eq!(asset.asset_code.0, *b"USDC");
                    assert_eq!(asset.issuer, ISSUER.parse().unwrap());
                }
                xdr::Asset::CreditAlphanum12(asset) => {
                    assert_eq!(asset.asset_code.0, *b"LONGASSET123");
                    assert_eq!(asset.issuer, ISSUER.parse().unwrap());
                }
                xdr::Asset::Native => panic!("expected an issued asset"),
            }
        }
    }
}
