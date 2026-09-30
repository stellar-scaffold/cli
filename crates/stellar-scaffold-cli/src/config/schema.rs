//! Typed `scaffold.yml` (schema version 2), deserialized with serde-saphyr.
//!
//! Shape errors (unknown keys, wrong types, duplicate keys, bad YAML) come
//! from serde and are reported one at a time. Everything else is checked
//! afterwards against these types, reporting every problem at once.

use std::fmt;
use std::marker::PhantomData;
use std::path::PathBuf;

use serde::Deserialize;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde_saphyr::Spanned;

use super::SCHEMA_VERSION;
use super::diagnostic::{Code, Diagnostic, from_parse_error, span};
use super::source::SourceType;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    /// Checked before the rest of the file is parsed; see [`parse`].
    #[serde(default)]
    pub version: Option<u64>,
    #[serde(default)]
    pub project: Project,
    /// Extensions in run order, each with its optional config.
    #[serde(default)]
    pub extensions: SpannedMap<Option<Value>>,
    #[serde(default)]
    pub networks: SpannedMap<NetworkDef>,
    #[serde(default)]
    pub contracts: SpannedMap<ContractSettings>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Project {
    pub contracts_dir: PathBuf,
    pub clients_dir: PathBuf,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            contracts_dir: "contracts".into(),
            clients_dir: "app-lib/clients".into(),
        }
    }
}

/// A network as written, before `extends` and built-in defaults are applied.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct NetworkDef {
    /// Another declared network to inherit from.
    pub extends: Option<Spanned<String>>,
    /// Defaults to stellar-cli's URL for the built-in names (except mainnet,
    /// which has none). Required otherwise, unless inherited.
    pub rpc_url: Option<Spanned<String>>,
    /// Defaults to stellar-cli's passphrase for the built-in names. Required
    /// otherwise, unless inherited.
    pub network_passphrase: Option<Spanned<String>>,
    pub rpc_headers: SpannedMap<Spanned<String>>,
    /// Account aliases Scaffold creates (and funds, where possible).
    pub accounts: Option<Vec<Spanned<String>>>,
    /// Signs deploys when `accounts` has several entries. Defaults to the
    /// first entry.
    pub default_account: Option<Spanned<String>>,
    /// Whether `build` starts a local quickstart container for this network.
    /// When unset, true only if the resolved passphrase is the standalone
    /// (local) one. Set it explicitly to override, e.g. `false` when a chain
    /// is already running at `rpc-url`.
    pub start_container: Option<Spanned<bool>>,
    /// Whether Scaffold may deploy `workspace` contracts here. When unset,
    /// false if the resolved passphrase is testnet's, futurenet's or
    /// mainnet's, true otherwise. Set it explicitly to override. An explicit
    /// value is inherited through `extends`; an unset one is recomputed from
    /// each network's own passphrase.
    pub allow_deploy: Option<Spanned<bool>>,
}

/// A contract entry. At the top level its keys are defaults for every
/// network it lists under `networks:`; entries there override them.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct ContractSettings {
    #[serde(rename = "type")]
    pub ty: Option<Spanned<SourceType>>,
    pub source: Option<Spanned<String>>,
    pub from_network: Option<Spanned<String>>,
    pub signer: Option<Spanned<String>>,
    /// Constructor arguments by contract parameter name.
    pub args: Option<Spanned<SpannedMap<Spanned<Value>>>>,
    pub after_deploy: Option<Spanned<Vec<Spanned<String>>>>,
    pub after_deploy_script: Option<Spanned<String>>,
    /// Per-network overrides. Only valid at the top level of a contract.
    pub networks: Option<Spanned<SpannedMap<ContractSettings>>>,
}

impl ContractSettings {
    /// Keys that only make sense when Scaffold deploys the contract, with
    /// where each was written.
    pub fn deploy_keys(&self) -> Vec<(&'static str, super::diagnostic::Span)> {
        [
            ("signer", self.signer.as_ref().map(span)),
            ("args", self.args.as_ref().map(span)),
            ("after-deploy", self.after_deploy.as_ref().map(span)),
            (
                "after-deploy-script",
                self.after_deploy_script.as_ref().map(span),
            ),
        ]
        .into_iter()
        .filter_map(|(k, s)| s.map(|s| (k, s)))
        .collect()
    }
}

/// A YAML mapping that keeps insertion order and where each key was written.
/// serde-saphyr rejects duplicate keys before this sees them.
#[derive(Debug, Clone)]
pub struct SpannedMap<V>(pub Vec<(Spanned<String>, V)>);

impl<V> Default for SpannedMap<V> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<V> SpannedMap<V> {
    pub fn get(&self, key: &str) -> Option<&V> {
        self.entry(key).map(|(_, v)| v)
    }

    pub fn entry(&self, key: &str) -> Option<(&Spanned<String>, &V)> {
        self.0
            .iter()
            .find(|(k, _)| k.value == key)
            .map(|(k, v)| (k, v))
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.entry(key).is_some()
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(k, _)| k.value.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Spanned<String>, &V)> {
        self.0.iter().map(|(k, v)| (k, v))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// How a map value is read. Section entries treat a bare `name:` (null) as
/// an empty entry, so `testnet:` means the same as `testnet: {}`.
pub trait MapValue<'de>: Sized {
    fn next<A: MapAccess<'de>>(map: &mut A) -> Result<Self, A::Error>;
}

macro_rules! null_is_default {
    ($($t:ty),*) => {$(
        impl<'de> MapValue<'de> for $t {
            fn next<A: MapAccess<'de>>(map: &mut A) -> Result<Self, A::Error> {
                Ok(map.next_value::<Option<Self>>()?.unwrap_or_default())
            }
        }
    )*};
}
null_is_default!(NetworkDef, ContractSettings);

macro_rules! plain {
    ($($t:ty),*) => {$(
        impl<'de> MapValue<'de> for $t {
            fn next<A: MapAccess<'de>>(map: &mut A) -> Result<Self, A::Error> {
                map.next_value()
            }
        }
    )*};
}
plain!(Spanned<String>, Spanned<Value>, Option<Value>);

impl<'de, V: MapValue<'de>> Deserialize<'de> for SpannedMap<V> {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct MapVisitor<V>(PhantomData<V>);
        impl<'de, V: MapValue<'de>> Visitor<'de> for MapVisitor<V> {
            type Value = SpannedMap<V>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a mapping")
            }
            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(SpannedMap::default())
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::new();
                while let Some(key) = map.next_key::<Spanned<String>>()? {
                    entries.push((key, V::next(&mut map)?));
                }
                Ok(SpannedMap(entries))
            }
        }
        d.deserialize_map(MapVisitor(PhantomData))
    }
}

/// An untyped YAML value: constructor arguments and extension config.
///
/// Integers keep their source text so values wider than 64 bits (`i128`,
/// `u256` arguments) survive; see [`restore_ints`].
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(String),
    Float(f64),
    Str(String),
    Seq(Vec<Spanned<Value>>),
    Map(SpannedMap<Spanned<Value>>),
}

impl PartialEq for SpannedMap<Spanned<Value>> {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self
                .0
                .iter()
                .zip(&other.0)
                .all(|((ka, va), (kb, vb))| ka.value == kb.value && va.value == vb.value)
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct ValueVisitor;
        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any YAML value")
            }
            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }
            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }
            fn visit_bool<E>(self, b: bool) -> Result<Value, E> {
                Ok(Value::Bool(b))
            }
            fn visit_i64<E>(self, n: i64) -> Result<Value, E> {
                Ok(Value::Int(n.to_string()))
            }
            fn visit_u64<E>(self, n: u64) -> Result<Value, E> {
                Ok(Value::Int(n.to_string()))
            }
            fn visit_f64<E>(self, n: f64) -> Result<Value, E> {
                Ok(Value::Float(n))
            }
            fn visit_str<E>(self, s: &str) -> Result<Value, E> {
                Ok(Value::Str(s.to_string()))
            }
            fn visit_string<E>(self, s: String) -> Result<Value, E> {
                Ok(Value::Str(s))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(Value::Seq(items))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
                let mut entries = Vec::new();
                while let Some(key) = map.next_key::<Spanned<String>>()? {
                    entries.push((key, map.next_value()?));
                }
                Ok(Value::Map(SpannedMap(entries)))
            }
        }
        d.deserialize_any(ValueVisitor)
    }
}

/// Untyped YAML numbers wider than 64 bits reach serde as lossy `f64`s.
/// Re-read each float's source text and keep integer literals exact.
fn restore_ints(value: &mut Spanned<Value>, source: &str) {
    match &mut value.value {
        Value::Float(_) => {
            let loc = value.referenced.span();
            let raw = loc
                .byte_offset()
                .zip(loc.byte_len())
                .and_then(|(start, len)| {
                    let start = usize::try_from(start).ok()?;
                    source.get(start..start + usize::try_from(len).ok()?)
                });
            if let Some(raw) = raw {
                let digits = raw.strip_prefix(['-', '+']).unwrap_or(raw);
                if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
                    value.value = Value::Int(raw.to_string());
                }
            }
        }
        Value::Seq(items) => items.iter_mut().for_each(|v| restore_ints(v, source)),
        Value::Map(map) => map.0.iter_mut().for_each(|(_, v)| restore_ints(v, source)),
        _ => {}
    }
}

fn restore_contract_ints(settings: &mut ContractSettings, source: &str) {
    if let Some(args) = &mut settings.args {
        args.value
            .0
            .iter_mut()
            .for_each(|(_, v)| restore_ints(v, source));
    }
    if let Some(networks) = &mut settings.networks {
        for (_, s) in &mut networks.value.0 {
            restore_contract_ints(s, source);
        }
    }
}

/// Only `version` matters before the full parse; everything else is ignored.
#[derive(Deserialize)]
struct VersionProbe {
    #[serde(default)]
    version: Option<Spanned<Value>>,
}

/// Parse `source`. Checks `version` first, so a file in another schema
/// version gets one clear error instead of one per unfamiliar key.
pub fn parse(source: &str) -> Result<Config, Diagnostic> {
    if source.trim().is_empty() {
        return Err(
            Diagnostic::error(Code::SchemaVersion, "scaffold.yml is empty")
                .help(format!("start the file with `version: {SCHEMA_VERSION}`")),
        );
    }
    let probe: VersionProbe =
        serde_saphyr::from_str_with_options(source, options()).map_err(|e| from_parse_error(&e))?;
    check_version(probe.version.as_ref())?;
    let mut config: Config =
        serde_saphyr::from_str_with_options(source, options()).map_err(|e| from_parse_error(&e))?;
    for (_, contract) in &mut config.contracts.0 {
        restore_contract_ints(contract, source);
    }
    Ok(config)
}

fn options() -> serde_saphyr::Options {
    let mut options = serde_saphyr::Options::default();
    // Diagnostics render their own snippets.
    options.with_snippet = false;
    options
}

fn check_version(version: Option<&Spanned<Value>>) -> Result<(), Diagnostic> {
    let Some(version) = version else {
        return Err(
            Diagnostic::error(Code::SchemaVersion, "scaffold.yml has no `version`")
                .help(format!("add `version: {SCHEMA_VERSION}` at the top")),
        );
    };
    let found = match &version.value {
        Value::Int(v) => v.parse::<u64>().ok(),
        _ => None,
    };
    match found {
        Some(v) if v == u64::from(SCHEMA_VERSION) => Ok(()),
        Some(1) => Err(Diagnostic::error(
            Code::SchemaVersion,
            "this is a version 1 scaffold.yml; `check` validates version 2",
        )
        .at(span(version))
        .help("version 1 is still read by `build`; version 2 moves `config:` to `project:` and adds `networks:` and `contracts:`")),
        Some(v) => Err(Diagnostic::error(
            Code::SchemaVersion,
            format!("schema version {v} is not supported; this CLI supports version {SCHEMA_VERSION}"),
        )
        .at(span(version))
        .help("upgrade stellar-scaffold-cli")),
        None => Err(Diagnostic::error(Code::SchemaVersion, "`version` must be an integer").at(span(version))),
    }
}

/// The `version` a `scaffold.yml` declares, if it parses far enough to say.
pub fn declared_version(source: &str) -> Option<u64> {
    let probe: VersionProbe = serde_saphyr::from_str_with_options(source, options()).ok()?;
    match probe.version?.value {
        Value::Int(v) => v.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(src: &str) -> Config {
        parse(src).unwrap_or_else(|d| panic!("{d:?}"))
    }

    #[test]
    fn null_entries_are_empty() {
        let c = parse_ok("version: 2\nnetworks:\n  testnet:\n  local: {}\n");
        assert_eq!(
            c.networks.keys().collect::<Vec<_>>(),
            vec!["testnet", "local"]
        );
    }

    #[test]
    fn keys_carry_spans() {
        let c = parse_ok("version: 2\nnetworks:\n  local: {}\n");
        let (key, _) = c.networks.entry("local").unwrap();
        assert_eq!(
            span(key),
            super::super::diagnostic::Span {
                line: 3,
                col: 3,
                len: 5
            }
        );
    }

    #[test]
    fn wide_integers_keep_their_text() {
        let c = parse_ok(
            "version: 2\ncontracts:\n  c:\n    args: { n: 170141183460469231731687303715884105727, f: 1.5, s: '7', l: [1, 99999999999999999999999] }\n",
        );
        let args = &c.contracts.get("c").unwrap().args.as_ref().unwrap().value;
        assert_eq!(
            args.get("n").unwrap().value,
            Value::Int("170141183460469231731687303715884105727".into())
        );
        assert_eq!(args.get("f").unwrap().value, Value::Float(1.5));
        assert_eq!(args.get("s").unwrap().value, Value::Str("7".into()));
        let Value::Seq(l) = &args.get("l").unwrap().value else {
            panic!()
        };
        assert_eq!(l[1].value, Value::Int("99999999999999999999999".into()));
    }

    #[test]
    fn unknown_keys_suggest_names() {
        let d = parse("version: 2\nnetworks:\n  local:\n    rpc_url: x\n").unwrap_err();
        assert_eq!(d.code, Code::UnknownKey);
        assert_eq!(d.help.as_deref(), Some("did you mean `rpc-url`?"));
        assert_eq!(d.span.map(|s| (s.line, s.col)), Some((4, 5)));
    }

    #[test]
    fn moved_keys_explain_the_replacement() {
        let d = parse("version: 2\nnetworks:\n  local:\n    run_locally: true\n").unwrap_err();
        assert!(d.help.unwrap().contains("start-container"));
    }

    #[test]
    fn optimize_points_at_the_build_flag() {
        let d = parse("version: 2\nproject:\n  optimize: false\n").unwrap_err();
        assert_eq!(d.code, Code::UnknownKey);
        assert!(d.help.unwrap().contains("--optimize=false"));
    }

    #[test]
    fn unknown_type_suggests_a_type() {
        let d = parse("version: 2\ncontracts:\n  c:\n    type: contrct\n").unwrap_err();
        assert_eq!(d.code, Code::InvalidSource);
        assert_eq!(d.help.as_deref(), Some("did you mean `contract`?"));
    }

    #[test]
    fn duplicate_keys_are_errors() {
        let d = parse("version: 2\nnetworks: {}\nnetworks: {}\n").unwrap_err();
        assert_eq!(d.code, Code::DuplicateKey);
        assert_eq!(d.span.map(|s| s.line), Some(3));
    }

    #[test]
    fn version_is_checked_before_unknown_keys() {
        let d = parse("version: 1\nconfig:\n  contracts_dir: contracts\n").unwrap_err();
        assert_eq!(d.code, Code::SchemaVersion);
        assert_eq!(parse("config: {}\n").unwrap_err().code, Code::SchemaVersion);
        assert_eq!(declared_version("version: 1\n"), Some(1));
    }

    #[test]
    fn syntax_errors_carry_a_position() {
        let d = parse("version: 2\nnetworks: [1\n").unwrap_err();
        assert_eq!(d.code, Code::Parse);
        assert!(d.span.is_some());
    }
}
