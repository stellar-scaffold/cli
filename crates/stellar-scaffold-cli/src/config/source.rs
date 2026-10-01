//! Contract source kinds (the `type` key) and validation of `source` values.

use std::path::Path;

/// What a contract entry's `source` refers to. Set explicitly with `type`
/// rather than inferred from the shape of `source`, so a typo in `source`
/// fails as "not a valid X" instead of silently becoming a different kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceType {
    /// A crate in `contracts-dir`, built and deployed by Scaffold.
    Workspace,
    /// An already-deployed contract, by `C…` address.
    Contract,
    /// A contract deployed via the Stellar Registry: `[namespace/]name[@version]`.
    Registry,
    /// A classic asset's Stellar Asset Contract: `CODE:ISSUER` or `native`.
    Asset,
    /// A local Wasm file. Types-only client, no contract ID.
    WasmFile,
    /// A Wasm hash installed on the network. Types-only client.
    WasmHash,
    /// A Wasm published to the Stellar Registry. Types-only client.
    WasmRegistry,
}

impl SourceType {
    pub const ALL: [(&'static str, Self); 7] = [
        ("workspace", Self::Workspace),
        ("contract", Self::Contract),
        ("registry", Self::Registry),
        ("asset", Self::Asset),
        ("wasm-file", Self::WasmFile),
        ("wasm-hash", Self::WasmHash),
        ("wasm-registry", Self::WasmRegistry),
    ];

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(name, _)| *name == s)
            .map(|(_, t)| *t)
    }

    pub fn as_str(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(_, t)| *t == self)
            .map(|(n, _)| *n)
            .expect("every SourceType is listed in ALL")
    }

    /// Whether `from-network` may be used: only kinds that name something
    /// already on another network whose Wasm can be fetched.
    pub fn allows_from_network(self) -> bool {
        matches!(self, Self::Contract | Self::Registry | Self::WasmHash)
    }

    /// Whether the kind names a third-party artifact that can move under you
    /// unless a version is pinned.
    fn is_versioned(self) -> bool {
        matches!(self, Self::Registry | Self::WasmRegistry)
    }
}

/// Whether two crate names refer to the same package. Cargo treats `-` and
/// `_` in package names as equivalent, and older configs used the
/// underscored form of hyphenated names.
pub fn same_crate(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.chars()
            .zip(b.chars())
            .all(|(x, y)| x == y || matches!((x, y), ('-', '_') | ('_', '-')))
}

/// Outcome of validating a `source` value against its `type`.
#[derive(Debug, PartialEq, Eq)]
pub enum Check {
    Ok,
    /// Valid, but a third-party source has no pinned `@version`.
    Unpinned,
    /// A `wasm-file` path that does not exist yet (it may be a build output).
    MissingFile,
    Invalid(String),
}

/// Validate `source` for `ty`. `root` resolves `wasm-file` paths. Crate
/// existence for `workspace` is checked separately against cargo metadata.
pub fn check(ty: SourceType, source: &str, root: &Path) -> Check {
    let result = match ty {
        SourceType::Workspace => check_crate_name(source),
        SourceType::Contract => stellar_strkey::Contract::from_string(source)
            .map(|_| ())
            .map_err(|_| format!("`{source}` is not a valid contract address (`C…` strkey)")),
        SourceType::Registry | SourceType::WasmRegistry => check_registry(source),
        SourceType::Asset => check_asset(source),
        SourceType::WasmHash => {
            if source.len() == 64 && source.chars().all(|c| c.is_ascii_hexdigit()) {
                Ok(())
            } else {
                Err(format!("`{source}` is not a Wasm hash (64 hex characters)"))
            }
        }
        SourceType::WasmFile => {
            let is_wasm = Path::new(source)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("wasm"));
            if !is_wasm {
                Err(format!("`{source}` is not a `.wasm` file"))
            } else if !root.join(source).is_file() {
                return Check::MissingFile;
            } else {
                Ok(())
            }
        }
    };
    match result {
        Err(e) => Check::Invalid(e),
        Ok(()) if ty.is_versioned() && !source.contains('@') => Check::Unpinned,
        Ok(()) => Check::Ok,
    }
}

fn check_crate_name(name: &str) -> Result<(), String> {
    let valid = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && name.starts_with(|c: char| c.is_ascii_alphabetic());
    if valid {
        Ok(())
    } else {
        Err(format!("`{name}` is not a valid crate name"))
    }
}

const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
    "where", "while",
];

/// Registry names: start with a letter, ASCII alphanumerics plus `-`/`_`,
/// at most 64 characters, not a Rust keyword. Same rules for the namespace.
fn check_registry_name(part: &str, what: &str) -> Result<(), String> {
    let valid = !part.is_empty()
        && part.len() <= 64
        && part.starts_with(|c: char| c.is_ascii_alphabetic())
        && part
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && !RUST_KEYWORDS.contains(&part.to_lowercase().as_str());
    if valid {
        Ok(())
    } else {
        Err(format!(
            "`{part}` is not a valid registry {what} (letter first, then letters, digits, `-` or `_`; at most 64 characters; not a Rust keyword)"
        ))
    }
}

fn check_registry(source: &str) -> Result<(), String> {
    let (path, version) = match source.split_once('@') {
        Some((path, version)) => (path, Some(version)),
        None => (source, None),
    };
    let name = match path.split_once('/') {
        Some((namespace, name)) => {
            check_registry_name(namespace, "namespace")?;
            name
        }
        None => path,
    };
    check_registry_name(name, "name")?;
    if let Some(version) = version {
        semver::Version::parse(version)
            .map_err(|e| format!("`{version}` is not a valid version: {e}"))?;
    }
    Ok(())
}

fn check_asset(source: &str) -> Result<(), String> {
    if source == "native" {
        return Ok(());
    }
    let Some((code, issuer)) = source.split_once(':') else {
        return Err(format!(
            "`{source}` is not an asset; expected `CODE:ISSUER` or `native`"
        ));
    };
    if code.is_empty() || code.len() > 12 || !code.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(format!(
            "`{code}` is not a valid asset code (1–12 letters or digits)"
        ));
    }
    stellar_strkey::ed25519::PublicKey::from_string(issuer)
        .map(|_| ())
        .map_err(|_| format!("`{issuer}` is not a valid issuer account (`G…` strkey)"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
    const CONTRACT: &str = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";

    fn check_str(ty: SourceType, s: &str) -> Check {
        check(ty, s, Path::new("/nonexistent"))
    }

    #[test]
    fn crate_names_ignore_dash_underscore() {
        assert!(same_crate(
            "nft-enumerable-example",
            "nft_enumerable_example"
        ));
        assert!(same_crate("guess_the-number", "guess-the_number"));
        assert!(!same_crate("nft-enumerable", "nft-enumerable-example"));
        assert!(!same_crate("nft.enumerable", "nft-enumerable"));
    }

    #[test]
    fn type_names_round_trip() {
        for (name, ty) in SourceType::ALL {
            assert_eq!(SourceType::parse(name), Some(ty));
            assert_eq!(ty.as_str(), name);
        }
        assert_eq!(SourceType::parse("crate"), None);
    }

    #[test]
    fn contract_requires_valid_strkey() {
        assert_eq!(check_str(SourceType::Contract, CONTRACT), Check::Ok);
        let mut typo = CONTRACT.to_string();
        typo.replace_range(5..6, "A");
        assert!(matches!(
            check_str(SourceType::Contract, &typo),
            Check::Invalid(_)
        ));
    }

    #[test]
    fn asset_forms() {
        assert_eq!(check_str(SourceType::Asset, "native"), Check::Ok);
        assert_eq!(
            check_str(SourceType::Asset, &format!("USDC:{ISSUER}")),
            Check::Ok
        );
        assert!(matches!(
            check_str(SourceType::Asset, "USDC"),
            Check::Invalid(_)
        ));
        assert!(matches!(
            check_str(SourceType::Asset, &format!("TOOLONGASSETCODE:{ISSUER}")),
            Check::Invalid(_)
        ));
        assert!(matches!(
            check_str(SourceType::Asset, "USDC:GXYZ"),
            Check::Invalid(_)
        ));
    }

    #[test]
    fn registry_forms() {
        assert_eq!(
            check_str(SourceType::Registry, "test-usdc@0.1.0"),
            Check::Ok
        );
        assert_eq!(
            check_str(SourceType::Registry, "unverified/my_c@1.2.3"),
            Check::Ok
        );
        assert_eq!(
            check_str(SourceType::Registry, "test-usdc"),
            Check::Unpinned
        );
        assert!(matches!(
            check_str(SourceType::Registry, "1abc"),
            Check::Invalid(_)
        ));
        assert!(matches!(
            check_str(SourceType::Registry, "fn@1.0.0"),
            Check::Invalid(_)
        ));
        assert!(matches!(
            check_str(SourceType::Registry, "abc@one"),
            Check::Invalid(_)
        ));
    }

    #[test]
    fn wasm_hash_and_file() {
        assert_eq!(check_str(SourceType::WasmHash, &"a".repeat(64)), Check::Ok);
        assert!(matches!(
            check_str(SourceType::WasmHash, "abc"),
            Check::Invalid(_)
        ));
        assert_eq!(
            check_str(SourceType::WasmFile, "target/x.wasm"),
            Check::MissingFile
        );
        assert!(matches!(
            check_str(SourceType::WasmFile, "x.txt"),
            Check::Invalid(_)
        ));
    }
}
