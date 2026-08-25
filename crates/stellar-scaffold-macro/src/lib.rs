#![recursion_limit = "128"]
extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::env;
use stellar_build::Network;
use syn::parse::{Parse, ParseStream, Result};
use syn::{Ident, LitStr, parse_macro_input};

mod asset;

pub(crate) fn manifest() -> std::path::PathBuf {
    std::path::PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("failed to find cargo manifest"))
        .join("Cargo.toml")
}

/// Generates a contract Client for a given contract.
/// The name should match a published contract or a contract in your current workspace.
///
/// # Usage
///
/// ```ignore
/// // For simple names (workspace contracts or registry names without hyphens):
/// import_contract_client!(registry);
///
/// // For hyphenated names or channel-prefixed registry paths:
/// import_contract_client!("unverified/guess-the-number");
///
/// // For specific versions, use quotes. `v` is optional:
/// import_contract_client!("registry@v1.0.0");
/// ```
///
/// When using a string literal, the module name is derived from the contract
/// name with hyphens replaced by underscores (e.g., `guess_the_number`).
///
/// # Panics
///
/// This function may panic in the following situations:
/// - If `stellar_build::get_target_dir()` fails to retrieve the target directory
/// - If the input tokens cannot be parsed as a valid identifier
/// - If the input tokens cannot be parsed as a valid identifier or string literal
/// - If the directory path cannot be canonicalized
/// - If the canonical path cannot be converted to a string
#[proc_macro]
pub fn import_contract_client(wasm_binary: TokenStream) -> TokenStream {
    let WasmBinary { mod_name, file } = parse_macro_input!(wasm_binary as WasmBinary);

    quote! {
        pub(crate) mod #mod_name {
            #![allow(clippy::ref_option, clippy::too_many_arguments)]
            use super::soroban_sdk;
            soroban_sdk::contractimport!(file = #file);
        }
    }
    .into()
}

struct WasmBinary {
    pub mod_name: Ident,
    pub file: String,
}

impl Parse for WasmBinary {
    fn parse(input: ParseStream) -> Result<Self> {
        let (lookup_name, mod_name, version) = parse_name_and_version(input)?;
        let wasm_path = resolve_wasm_path(&lookup_name, &mod_name, version.as_deref())?;
        let file = wasm_path.display().to_string();
        Ok(Self { mod_name, file })
    }
}

/// Parse a contract name with an optional version specifier.
///
/// Accepts an identifier like `registry` or a string like
/// `"unverified/guess-the-number"`, optionally followed by `@VERSION`
/// (e.g. `"registry@v1.4.0"` or `"registry@1.4.0"`). Returns the bare
/// lookup name, a sanitized module identifier, and the parsed version
/// (with any leading `v` stripped).
fn parse_name_and_version(input: ParseStream) -> Result<(String, Ident, Option<String>)> {
    let span = input.span();
    let raw = if input.peek(LitStr) {
        input.parse::<LitStr>()?.value()
    } else {
        input.parse::<Ident>()?.to_string()
    };

    if regex::Regex::new(r"(^/)|(/$)").unwrap().is_match(&raw) {
        return Err(syn::Error::new(
            span,
            format!("bad leading/trailing slash: `{raw}`"),
        ));
    }

    // Split off optional version: "name@v1.4.0" or "name@1.4.0"
    let (name_part, version) = match raw.split_once('@') {
        Some((name, ver)) => {
            let ver = ver.strip_prefix('v').unwrap_or(ver);
            (name.to_string(), Some(ver.to_string()))
        }
        None => (raw, None),
    };

    // Derive a valid Rust identifier for the module name (no version)
    // e.g. "unverified/guess-the-number" -> "guess_the_number"
    let mod_name_str = name_part
        .rsplit('/')
        .next()
        .unwrap_or(&name_part)
        .replace('-', "_");
    let mod_name = Ident::new(&mod_name_str, span);

    Ok((name_part, mod_name, version))
}

fn build_local_wasm_path(
    target_dir: &std::path::Path,
    mod_name: &Ident,
    version: Option<&str>,
) -> std::path::PathBuf {
    let file_stem = match version {
        Some(v) => format!("{}_{}", mod_name, v.replace('.', "_")),
        None => mod_name.to_string(),
    };
    target_dir.join(file_stem).with_extension("wasm")
}

fn resolve_wasm_path(
    lookup_name: &str,
    mod_name: &Ident,
    version: Option<&str>,
) -> Result<std::path::PathBuf> {
    let target_dir = stellar_build::get_target_dir(&manifest()).unwrap();
    let local_path = build_local_wasm_path(&target_dir, mod_name, version);

    // 1. Check local build target
    if local_path.exists() {
        return Ok(local_path.canonicalize().expect("canonicalize failed"));
    }

    // 2. If STELLAR_NO_REGISTRY set to 1, error
    if let Ok(v) = env::var("STELLAR_NO_REGISTRY")
        && &v == "1"
    {
        return Err(syn::Error::new(
            mod_name.span(),
            format!(
                "No local wasm found and STELLAR_NO_REGISTRY=1 so not checking Registry. \
                Download manually with `stellar registry download {lookup_name}`"
            ),
        ));
    }

    // 3. if var absent or set to something else, try to download
    download_from_registry(lookup_name, &local_path, mod_name.span(), version)
}

fn download_from_registry(
    lookup_name: &str,
    local_path: &std::path::Path,
    span: Span,
    version: Option<&str>,
) -> Result<std::path::PathBuf> {
    // 1. create `target/stellar/[network]` directory, if not already present
    let parent = local_path.parent().expect("no parent");
    if !parent.exists() {
        std::fs::create_dir_all(parent).expect("creating parent directory failed");
    }

    // 2. download using `stellar registry download`
    let mut args = vec![
        "registry".to_string(),
        "download".to_string(),
        lookup_name.to_string(),
        "--out-file".to_string(),
        local_path.display().to_string(),
    ];
    if let Some(v) = version {
        args.push("--version".to_string());
        args.push(v.to_string());
    }
    let status = std::process::Command::new("stellar")
        .args(&args)
        .status()
        .expect(
            "failed to execute `stellar registry download`; try `cargo install stellar-registry-cli` and try again",
        );

    // 3. check status
    if status.success() && local_path.exists() {
        Ok(local_path.canonicalize().expect("canonicalize failed"))
    } else {
        let local_path = local_path.display().to_string();
        Err(syn::Error::new(
            span,
            format!(
                "Could not find Wasm `{lookup_name}`. Checked: \
                \n\n• {local_path} \
                \n• `stellar registry download {lookup_name}` \
                \n\nYou can: \
                \n\n1. check the name & network and try again (https://stellar.rgstry.xyz) \
                \n2. add this Wasm to your local `target` directory manually \
                (perhaps by compiling a contract) \
                \n3. run `stellar registry download {lookup_name}` yourself. \
                \n\nSet STELLAR_NO_REGISTRY=1 to skip registry lookup."
            ),
        ))
    }
}

/// Generates a contract Client for a given asset.
/// It produces 2 modules that can be used either in your contract code or in unit tests.
/// As the first argument, it expects the name of an asset, e.g. "native" or "USDC:G1...."
/// To generate the contract id for the asset, it uses the `STELLAR_NETWORK` environment variable,
/// which could be either `local`, `testnet`, `futurenet` or `mainnet`. (uses `local` if not set)
///
/// Example:
/// ```ignore
/// import_asset!("native");
/// ```
/// Can be used in unit tests as follows:
/// ```ignore
/// let env = &Env::default();
/// let admin = &Address::generate(env);
/// let sac = test_native::register(env, admin);
/// let client = test_native::stellar_asset_client(env, &sac);
/// assert_eq!(client.admin(), *admin);
/// assert_eq!(client.balance(admin), 1000000000);
/// ```
/// And in your contract code:
/// ```ignore
/// ```
/// # Panics
///
#[proc_macro]
pub fn import_asset(input: TokenStream) -> TokenStream {
    // Parse the input as a string literal
    let input_str = syn::parse_macro_input!(input as syn::LitStr);
    asset::parse_literal(&input_str).into()
}

#[cfg(test)]
mod parse_name_and_version {
    use super::*;
    use syn::parse::Parser;

    #[test]
    fn parse_simple_name() {
        let (lookup_name, mod_name, _) = (|input: ParseStream| parse_name_and_version(input))
            .parse2(quote!(registry))
            .unwrap();
        assert_eq!(mod_name.to_string(), "registry");
        assert_eq!(lookup_name, "registry");
    }

    #[test]
    fn parse_channel_hyphenated_name() {
        let (lookup_name, mod_name, _) = (|input: ParseStream| parse_name_and_version(input))
            .parse2(quote!("guess-the-number"))
            .unwrap();
        assert_eq!(mod_name.to_string(), "guess_the_number");
        assert_eq!(lookup_name, "guess-the-number");
    }

    #[test]
    fn parse_channel_prefixed_name() {
        let (lookup_name, mod_name, _) = (|input: ParseStream| parse_name_and_version(input))
            .parse2(quote!("unverified/guess-the-number"))
            .unwrap();
        assert_eq!(mod_name.to_string(), "guess_the_number");
        assert_eq!(lookup_name, "unverified/guess-the-number");
    }

    #[test]
    fn parse_channel_simple_name() {
        let (lookup_name, mod_name, _) = (|input: ParseStream| parse_name_and_version(input))
            .parse2(quote!("unverified/hello"))
            .unwrap();
        assert_eq!(mod_name.to_string(), "hello");
        assert_eq!(lookup_name, "unverified/hello");
    }

    #[test]
    fn parse_underscored_name() {
        let (lookup_name, mod_name, _) = (|input: ParseStream| parse_name_and_version(input))
            .parse2(quote!("my_contract"))
            .unwrap();
        assert_eq!(mod_name, "my_contract");
        assert_eq!(lookup_name, "my_contract");
    }

    #[test]
    fn error_trailing_slash() {
        let err = (|input: ParseStream| parse_name_and_version(input))
            .parse2(quote!("unverified/"))
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("bad leading/trailing slash: `unverified/`"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn error_leading_slash() {
        let err = (|input: ParseStream| parse_name_and_version(input))
            .parse2(quote!("/guess-the-number"))
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("bad leading/trailing slash: `/guess-the-number`"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn multiple_slashes_returns_final_as_mod_name() {
        let (lookup_name, mod_name, _) = (|input: ParseStream| parse_name_and_version(input))
            .parse2(quote!("a/b/c"))
            .unwrap();
        assert_eq!(mod_name, "c");
        assert_eq!(lookup_name, "a/b/c");
    }

    #[test]
    #[should_panic(expected = "Ident is not allowed to be empty")]
    fn error_empty_string() {
        (|input: ParseStream| parse_name_and_version(input))
            .parse2(quote!(""))
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "not a valid Ident")]
    fn error_starts_with_digit() {
        (|input: ParseStream| parse_name_and_version(input))
            .parse2(quote!("123bad"))
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "not a valid Ident")]
    fn error_invalid_characters() {
        (|input: ParseStream| parse_name_and_version(input))
            .parse2(quote!("hello world"))
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "not a valid Ident")]
    fn error_channel_prefixed_starts_with_digit() {
        (|input: ParseStream| parse_name_and_version(input))
            .parse2(quote!("unverified/1bad"))
            .unwrap();
    }

    #[test]
    fn main_channel_with_version() {
        let (lookup_name, mod_name, version) = (|input: ParseStream| parse_name_and_version(input))
            .parse2(quote!("registry@v1.0.1"))
            .unwrap();
        assert_eq!(mod_name, "registry");
        assert_eq!(lookup_name, "registry");
        assert_eq!(&version.unwrap(), "1.0.1");
    }

    #[test]
    fn unverified_channel_with_version() {
        let (lookup_name, mod_name, version) = (|input: ParseStream| parse_name_and_version(input))
            .parse2(quote!("unverified/guess-the-number@0.4.0"))
            .unwrap();
        assert_eq!(mod_name, "guess_the_number");
        assert_eq!(lookup_name, "unverified/guess-the-number");
        assert_eq!(&version.unwrap(), "0.4.0");
    }

    #[test]
    fn prerelease_version() {
        let (lookup_name, mod_name, version) = (|input: ParseStream| parse_name_and_version(input))
            .parse2(quote!("registry@1.0.0-rc.1"))
            .unwrap();
        assert_eq!(mod_name, "registry");
        assert_eq!(lookup_name, "registry");
        assert_eq!(&version.unwrap(), "1.0.0-rc.1");
    }
}

#[cfg(test)]
mod test_build_local_wasm_path {
    use super::*;
    use std::path::Path;

    fn ident(string: &str) -> Ident {
        Ident::new(string, proc_macro2::Span::call_site())
    }

    #[test]
    fn includes_underscore_delimited_version() {
        let path = build_local_wasm_path(Path::new("target"), &ident("a"), Some("1.0.0"));
        assert_eq!(path, Path::new("target/a_1_0_0.wasm"));
    }

    #[test]
    fn no_version() {
        let path = build_local_wasm_path(Path::new("target"), &ident("registry"), None);
        assert_eq!(path, Path::new("target/registry.wasm"));
    }

    #[test]
    fn prerelease_version() {
        let path = build_local_wasm_path(Path::new("target"), &ident("foo"), Some("1.0.0-rc.1"));
        assert_eq!(path, Path::new("target/foo_1_0_0-rc_1.wasm"));
    }
}
