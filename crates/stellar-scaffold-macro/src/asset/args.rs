use std::fmt::Display;
use std::io::Cursor;

use stellar_xdr::curr as xdr;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitInt, Token};
use xdr::WriteXdr;

use crate::asset::is_native;

// Represents the full asset macro arguments.
#[derive(Debug)]
pub struct AssetArgs {
    pub asset: AssetSpec,
    pub decimals: u32,
}

impl Parse for AssetArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let asset: AssetSpec = input.parse()?;
        let decimals = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            input.parse::<LitInt>()?.base10_parse()?
        } else {
            7
        };
        Ok(AssetArgs { asset, decimals })
    }
}

impl TryFrom<proc_macro2::TokenStream> for AssetArgs {
    type Error = syn::Error;
    fn try_from(value: proc_macro2::TokenStream) -> Result<Self, Self::Error> {
        syn::parse2::<AssetArgs>(value)
    }
}

// The `asset` piece.
#[derive(Debug)]
pub enum AssetSpec {
    Native(Ident),
    Asset(Ident, Ident),
}

impl AssetSpec {
    pub fn code(&self) -> String {
        match self {
            AssetSpec::Native(ident) => ident.to_string(),
            AssetSpec::Asset(ident, _) => ident.to_string(),
        }
    }

    pub fn serialized_asset(&self) -> syn::Result<Vec<u8>> {
        let asset: xdr::Asset = self.try_into()?;
        let mut data = Vec::new();
        let cursor = Cursor::new(&mut data);
        let mut limit = xdr::Limited::new(cursor, xdr::Limits::none());
        asset.write_xdr(&mut limit).unwrap();
        Ok(data)
    }
}

impl Display for AssetSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetSpec::Native(ident) => write!(f, "{ident}"),
            AssetSpec::Asset(code, asset) => write!(f, "{code}:{asset}"),
        }
    }
}

impl Parse for AssetSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let symbol: Ident = input.parse()?;
        Ok(if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            AssetSpec::Asset(symbol, input.parse()?)
        } else {
            if !is_native(&symbol.to_string()) {
                return Err(syn::Error::new(
                    symbol.span(),
                    format!("Native asset must be \"xlm\" or \"native\". Found \"{symbol}\""),
                ));
            }
            AssetSpec::Native(symbol)
        })
    }
}

impl TryFrom<&AssetSpec> for xdr::Asset {
    type Error = syn::Error;

    fn try_from(value: &AssetSpec) -> Result<Self, Self::Error> {
        let AssetSpec::Asset(code, issuer) = value else {
            return Ok(xdr::Asset::Native);
        };
        let issuer: xdr::AccountId = issuer
            .to_string()
            .parse()
            .map_err(|_| syn::Error::new(issuer.span(), "invalid account id"))?;
        let re = regex::Regex::new("^[[:alnum:]]{1,12}$").expect("regex failed");
        let code_str = code.to_string();
        if !re.is_match(&code_str) {
            return Err(syn::Error::new(
                code.span(),
                "invalid asset code \"{code_str}\"",
            ));
        }
        let asset_code = match code_str.len() {
            4 => xdr::AssetCode::CreditAlphanum4(xdr::AssetCode4(
                code_str.as_bytes().try_into().unwrap(),
            )),
            12 => xdr::AssetCode::CreditAlphanum12(xdr::AssetCode12(
                code_str.as_bytes().try_into().unwrap(),
            )),
            _ => {
                return Err(syn::Error::new(
                    code.span(),
                    "invalid asset code length \"{code_str}\". Must be 4 or twelve",
                ));
            }
        };
        Ok(match asset_code {
            xdr::AssetCode::CreditAlphanum4(asset_code) => {
                xdr::Asset::CreditAlphanum4(xdr::AlphaNum4 { asset_code, issuer })
            }
            xdr::AssetCode::CreditAlphanum12(asset_code) => {
                xdr::Asset::CreditAlphanum12(xdr::AlphaNum12 { asset_code, issuer })
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quote::quote;
    #[test]
    fn parse_native() {
        let _: AssetArgs = quote! { native }.try_into().unwrap();
    }
    #[test]
    fn parse_asset() {
        let _: AssetArgs =
            quote! { USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN, 8 }
                .try_into()
                .unwrap();
    }
}
