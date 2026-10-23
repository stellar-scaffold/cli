use proc_macro2::TokenStream;
use std::io::Cursor;
use std::str::FromStr;

use stellar_xdr::curr as xdr;
use xdr::WriteXdr;

use quote::{format_ident, quote};

fn is_native(s: &str) -> bool {
    s == "native" || s == "xlm"
}

pub fn parse_asset(s: &str) -> Result<(xdr::Asset, String), xdr::Error> {
    if is_native(s) {
        return Ok((xdr::Asset::Native, s.to_string()));
    }
    let split: Vec<&str> = s.splitn(2, ':').collect();
    assert!(split.len() == 2, "invalid asset \"{s}\"");
    let code = split[0];
    let iss = split[1];

    let issuer: xdr::AccountId = xdr::AccountId::from_str(iss)?;
    let re = regex::Regex::new("^[[:alnum:]]{1,12}$").expect("regex failed");
    assert!(re.is_match(code), "invalid asset \"{s}\"");
    let asset_code = match code.len() {
        4 => xdr::AssetCode::CreditAlphanum4(xdr::AssetCode4(code.as_bytes().try_into()?)),
        12 => xdr::AssetCode::CreditAlphanum12(xdr::AssetCode12(code.as_bytes().try_into()?)),
        _ => panic!("invalid asset code length"),
    };
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

pub fn get_serialized_asset(asset: &str) -> Result<(String, Vec<u8>), xdr::Error> {
    let (asset, code) = parse_asset(asset)?;
    let mut data = Vec::new();
    let cursor = Cursor::new(&mut data);
    let mut limit = xdr::Limited::new(cursor, xdr::Limits::none());
    asset.write_xdr(&mut limit)?;
    Ok((code, data))
}

/// Generate the code to read the `STELLAR_NETWORK` environment variable
/// and call the `generate_asset_id` function
pub fn parse_literal(lit_str: &syn::LitStr) -> TokenStream {
    let (code, data) = get_serialized_asset(&lit_str.value()).unwrap();
    let mod_name = format_ident!("{code}");
    let size = data.len();
    let xlm_fns = (is_native(&code)).then(|| {
        quote! {
            /// 1 xlm in stroops
            const ONE: i128 = 1_000_000_0;
            pub const fn to_stroops(num: u64) -> i128 {
                (num as i128) * ONE
            }
        }
    });
    let key_name = format_ident!("{code}_KEY");
    quote! {
        mod #mod_name {
            pub const SERIALIZED_ASSET: [u8; #size] = [ #(#data),* ];
            /// Create a Stellar Asset Client for the asset which provides an admin interface
            pub fn stellar_asset_client<'a>(env: &soroban_sdk::Env) -> soroban_sdk::token::StellarAssetClient<'a> {
                soroban_sdk::token::StellarAssetClient::new(&env, &contract_id(env))
            }
            /// Create a Stellar Asset Client for the asset which provides an admin interface
            pub fn token_client<'a>(env: &soroban_sdk::Env) -> soroban_sdk::token::TokenClient<'a> {
                soroban_sdk::token::TokenClient::new(&env, &contract_id(env))
            }

            #xlm_fns

            #[allow(non_upper_case_globals)]
            #[cfg(test)]
            pub(crate) mod #mod_name {
                use super::*;
                const #key_name: &soroban_sdk::Symbol = &soroban_sdk::symbol_short!(#code);

                pub fn contract_id(env: &soroban_sdk::Env) -> soroban_sdk::Address {
                        env.storage()
                            .instance()
                            .get::<_, soroban_sdk::Address>(#key_name)
                            .expect("XLM contract not initialized. Please deploy the XLM contract first.")
                }


                /// Registers a new SAC contract (to use in unit tests only)
                pub fn register(env: &soroban_sdk::Env, admin: &soroban_sdk::Address) -> soroban_sdk::testutils::StellarAssetContract {
                    let sac = env.register_stellar_asset_contract_v2(admin.clone());
                    env.storage().instance().set(#key_name, &sac.address());
                    stellar_asset_client(env).mint(admin, &to_stroops(10_000));
                    sac
                }
            }
            #[cfg(not(test))]
            #[allow(non_upper_case_globals)]
            pub(crate) mod #mod_name {
                use super::*;
                /// Contract id for the Stellar Asset Contract
                pub fn contract_id(env: &soroban_sdk::Env) -> soroban_sdk::Address {
                    env.deployer().with_stellar_asset(SERIALIZED_ASSET).deployed_address()
                }

                pub fn register(env: &soroban_sdk::Env, _admin: &soroban_sdk::Address) {
                    if contract_id(env).executable().is_none()  {
                        env.deployer().with_stellar_asset(SERIALIZED_ASSET).deploy();
                    }
                }

                pub fn to_min_unit(float: f64) -> i128 {
                    return (float * 10_000_000_f64) as i128;
                }

                pub fn from_min_unit(num: i128) -> f64 {
                    return (num) as f64 / 10_000_000_f64;
                }
            }
            pub use #mod_name::*;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const USDC: &str = "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

    // Test for  parsing natve token
    #[test]
    fn parse_native() {
        let (asset, code) = parse_asset("native").unwrap();
        assert_eq!(asset, xdr::Asset::Native);
        assert_eq!(code, "native");
        let (asset, code) = parse_asset("xlm").unwrap();
        assert_eq!(asset, xdr::Asset::Native);
        assert_eq!(code, "xlm");
        for network in &NETWORKS {
            match (
                network,
                generate_asset_id("native", network)
                    .unwrap()
                    .0
                    .to_string()
                    .as_str(),
            ) {
                (Local, "CDMLFMKMMD7MWZP3FKUBZPVHTUEDLSX4BYGYKH4GCESXYHS3IHQ4EIG4")
                | (Testnet, "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC")
                | (Futurenet, "CB64D3G7SM2RTH6JSGG34DDTFTQ5CFDKVDZJZSODMCX4NJ2HV2KN7OHT")
                | (Mainnet, "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA") => {}
                (x, s) => panic!("Unexpected network {x:?} with asset {s}"),
            }
        }

        assert_eq!(get_serialized_asset("native").unwrap().0, "native");
        assert_eq!(get_serialized_asset("native").unwrap().1, [0, 0, 0, 0]);
    }

    // Test for parsing USDC token
    #[test]
    fn parse_usdc() {
        assert_eq!(get_serialized_asset(USDC).unwrap().0, "USDC");
        assert_eq!(
            get_serialized_asset(USDC).unwrap().1,
            [
                0, 0, 0, 1, 85, 83, 68, 67, 0, 0, 0, 0, 59, 153, 17, 56, 14, 254, 152, 139, 160,
                168, 144, 14, 177, 207, 228, 79, 54, 111, 125, 190, 148, 107, 237, 7, 114, 64, 247,
                246, 36, 223, 21, 197
            ]
        );
    }

    #[test]
    fn native_client() {
        let lit: syn::LitStr = syn::parse_quote!("native");
        let expected = quote! {
            #[allow(non_upper_case_globals)]
            pub(crate) mod test_native {
                use super::*;

                /// Create a Stellar Asset Client for the asset which provides an admin interface
                pub fn stellar_asset_client<'a>(env: &soroban_sdk::Env, sac:&soroban_sdk::testutils::StellarAssetContract) -> soroban_sdk::token::StellarAssetClient<'a> {
                    soroban_sdk::token::StellarAssetClient::new(&env, &sac.address())
                }
                /// Create a Stellar Asset Client for the asset which provides an admin interface
                pub fn token_client<'a>(env: &soroban_sdk::Env, sac:&soroban_sdk::testutils::StellarAssetContract) -> soroban_sdk::token::TokenClient<'a> {
                    soroban_sdk::token::TokenClient::new(&env, &sac.address())
                }

                /// Registers a new SAC contract (to use in unit tests only)
                pub fn register(env: &soroban_sdk::Env, admin: &soroban_sdk::Address) -> soroban_sdk::testutils::StellarAssetContract {
                    let sac = env.register_stellar_asset_contract_v2(admin.clone());
                    let cl = stellar_asset_client(env, &sac);
                    env.mock_all_auths();
                    cl.mint(admin, &1_000_000_000_i128);
                    sac
                }

                pub fn to_min_unit(float: f64) -> i128 {
                    return (float * 10_000_000_f64) as i128;
                }

                pub fn from_min_unit(num: i128) -> f64 {
                    return (num) as f64 / 10_000_000_f64;
                }
            }

            #[allow(non_upper_case_globals)]
            pub(crate) mod native {
                use super::*;
                pub const SERIALIZED_ASSET: [u8; 4usize] = [0u8 , 0u8 , 0u8 , 0u8];

                /// Contract id for the Stellar Asset Contract
                pub fn contract_id(env: &soroban_sdk::Env) -> soroban_sdk::Address {
                    env.deployer().with_stellar_asset(SERIALIZED_ASSET).deployed_address()
                }

                /// Create a Stellar Asset Client for the asset which provides an admin interface
                pub fn stellar_asset_client<'a>(env: &soroban_sdk::Env) -> soroban_sdk::token::StellarAssetClient<'a> {
                    soroban_sdk::token::StellarAssetClient::new(&env, &contract_id(env))
                }
                /// Create a Stellar Asset Client for the asset which provides an admin interface
                pub fn token_client<'a>(env: &soroban_sdk::Env) -> soroban_sdk::token::TokenClient<'a> {
                    soroban_sdk::token::TokenClient::new(&env, &contract_id(env))
                }

                pub fn register(env: &soroban_sdk::Env) {
                    let symbol = token_client(env).try_symbol();
                    if symbol.is_err()  {
                        env.deployer().with_stellar_asset(SERIALIZED_ASSET).deploy();
                    }
                }

                pub fn to_min_unit(float: f64) -> i128 {
                    return (float * 10_000_000_f64) as i128;
                }

                pub fn from_min_unit(num: i128) -> f64 {
                    return (num) as f64 / 10_000_000_f64;
                }
            }
        };
        let generated = parse_literal(&lit);
        assert_eq!(generated.to_string(), expected.to_string());

        let lit: syn::LitStr = syn::parse_quote!("xlm");
        let generated = parse_literal(&lit);
        assert_eq!(
            generated.to_string(),
            expected.to_string().replace("native", "xlm")
        );
    }

    #[test]
    fn usdc_client() {
        let lit: syn::LitStr =
            syn::parse_quote!("USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN");
        let expected = quote! {
            #[allow(non_upper_case_globals)]
            pub(crate) mod test_USDC {
                use super::*;

                /// Create a Stellar Asset Client for the asset which provides an admin interface
                pub fn stellar_asset_client<'a>(env: &soroban_sdk::Env, sac:&soroban_sdk::testutils::StellarAssetContract) -> soroban_sdk::token::StellarAssetClient<'a> {
                    soroban_sdk::token::StellarAssetClient::new(&env, &sac.address())
                }
                /// Create a Stellar Asset Client for the asset which provides an admin interface
                pub fn token_client<'a>(env: &soroban_sdk::Env, sac:&soroban_sdk::testutils::StellarAssetContract) -> soroban_sdk::token::TokenClient<'a> {
                    soroban_sdk::token::TokenClient::new(&env, &sac.address())
                }

                /// Registers a new SAC contract (to use in unit tests only)
                pub fn register(env: &soroban_sdk::Env, admin: &soroban_sdk::Address) -> soroban_sdk::testutils::StellarAssetContract {
                    let sac = env.register_stellar_asset_contract_v2(admin.clone());
                    let cl = stellar_asset_client(env, &sac);
                    env.mock_all_auths();
                    cl.mint(admin, &1_000_000_000_i128);
                    sac
                }

                pub fn to_min_unit(float: f64) -> i128 {
                    return (float * 10_000_000_f64) as i128;
                }

                pub fn from_min_unit(num: i128) -> f64 {
                    return (num) as f64 / 10_000_000_f64;
                }
            }

            #[allow(non_upper_case_globals)]
            pub(crate) mod USDC {
                use super::*;
                pub const SERIALIZED_ASSET: [u8 ; 44usize] = [0u8 , 0u8 , 0u8 , 1u8 , 85u8 , 83u8 , 68u8 , 67u8 , 0u8 , 0u8 , 0u8 , 0u8 , 59u8 , 153u8 , 17u8 , 56u8 , 14u8 , 254u8 , 152u8 , 139u8 , 160u8 , 168u8 , 144u8 , 14u8 , 177u8 , 207u8 , 228u8 , 79u8 , 54u8 , 111u8 , 125u8 , 190u8 , 148u8 , 107u8 , 237u8 , 7u8 , 114u8 , 64u8 , 247u8 , 246u8 , 36u8 , 223u8 , 21u8 , 197u8];

                /// Contract id for the Stellar Asset Contract
                pub fn contract_id(env: &soroban_sdk::Env) -> soroban_sdk::Address {
                    env.deployer().with_stellar_asset(SERIALIZED_ASSET).deployed_address()
                }

                /// Create a Stellar Asset Client for the asset which provides an admin interface
                pub fn stellar_asset_client<'a>(env: &soroban_sdk::Env) -> soroban_sdk::token::StellarAssetClient<'a> {
                    soroban_sdk::token::StellarAssetClient::new(&env, &contract_id(env))
                }
                /// Create a Stellar Asset Client for the asset which provides an admin interface
                pub fn token_client<'a>(env: &soroban_sdk::Env) -> soroban_sdk::token::TokenClient<'a> {
                    soroban_sdk::token::TokenClient::new(&env, &contract_id(env))
                }

                pub fn register(env: &soroban_sdk::Env) {
                    let symbol = token_client(env).try_symbol();
                    if symbol.is_err()  {
                        env.deployer().with_stellar_asset(SERIALIZED_ASSET).deploy();
                    }
                }

                pub fn to_min_unit(float: f64) -> i128 {
                    return (float * 10_000_000_f64) as i128;
                }

                pub fn from_min_unit(num: i128) -> f64 {
                    return (num) as f64 / 10_000_000_f64;
                }
            }
        };
        let generated = parse_literal(&lit);
        assert_eq!(generated.to_string(), expected.to_string());
    }
}
