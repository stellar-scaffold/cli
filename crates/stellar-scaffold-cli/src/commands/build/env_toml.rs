use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::BTreeMap as Map;
use std::path::Path;
use toml::value::Table;

use crate::commands::build::clients::ScaffoldEnv;

pub const ENV_FILE: &str = "environments.toml";

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("⛔ ️parsing environments.toml: {0}")]
    ParsingToml(#[from] toml::de::Error),
    #[error("⛔ ️no settings for current STELLAR_SCAFFOLD_ENV ({0:?}) found in environments.toml")]
    NoSettingsForCurrentEnv(String),
    #[error("⛔ ️reading environments.toml as a string: {0}")]
    ParsingString(#[from] std::io::Error),
}

type Environments = Map<Box<str>, Environment>;

/// A single extension entry parsed from `environments.toml`.
///
/// Extensions are declared with two independent keys that can be used together
/// or separately:
///
/// ```toml
/// [development]
/// extensions = ["reporter", "indexer"]   # execution order
///
/// [development.ext.reporter]             # optional per-extension config
/// warn_size_kb = 128
///
/// [development.ext.indexer]
/// storage = "sqlite"
/// events = ["transfer", "mint"]
/// ```
///
/// `extensions` controls which extensions run and in what order. `ext.<name>`
/// tables are optional — omitting one means that extension receives no config.
/// An extension listed in `ext` but absent from `extensions` is ignored.
#[derive(Debug, Clone)]
pub struct ExtensionEntry {
    /// Extension name as declared in `extensions = [...]` (matches the
    /// executable name the scaffold tool will invoke).
    pub name: String,
    /// Arbitrary JSON value parsed from the `[env.ext.<name>]` sub-table.
    /// `None` when no config table exists for this extension, or when its
    /// table is empty.
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub accounts: Option<Vec<Account>>,
    pub network: Network,
    pub contracts: Option<IndexMap<Box<str>, Contract>>,
    /// Extensions to invoke for this environment, in execution order.
    pub extensions: Vec<ExtensionEntry>,
}

fn deserialize_accounts<'de, D>(deserializer: D) -> Result<Option<Vec<Account>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<Vec<AccountRepresentation>> = Option::deserialize(deserializer)?;
    Ok(opt.map(|vec| vec.into_iter().map(Account::from).collect()))
}

impl<'de> Deserialize<'de> for Environment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct EnvironmentHelper {
            #[serde(default, deserialize_with = "deserialize_accounts")]
            accounts: Option<Vec<Account>>,
            network: Network,
            contracts: Option<Table>,
            /// Ordered list of extension names to invoke.
            #[serde(default)]
            extensions: Vec<String>,
            /// Per-extension config tables, keyed by extension name.
            ext: Option<Table>,
        }

        let helper = EnvironmentHelper::deserialize(deserializer)?;

        let contracts = helper
            .contracts
            .map(|contracts_table| {
                contracts_table
                    .into_iter()
                    .map(|(key, value)| {
                        let contract: Contract =
                            Contract::deserialize(value).map_err(serde::de::Error::custom)?;
                        Ok((key.into_boxed_str(), contract))
                    })
                    .collect::<Result<IndexMap<_, _>, D::Error>>()
            })
            .transpose()?;

        let extensions = parse_extensions(helper.extensions, helper.ext);

        Ok(Environment {
            accounts: helper.accounts,
            network: helper.network,
            contracts,
            extensions,
        })
    }
}

/// Zips the ordered `extensions` name list with the optional `ext` config
/// tables into a single [`ExtensionEntry`] list.
///
/// Extensions listed in `ext` but absent from `names` are silently ignored so
/// that leftover config tables don't cause errors when an extension is
/// temporarily removed from the run list.
fn parse_extensions(names: Vec<String>, ext: Option<Table>) -> Vec<ExtensionEntry> {
    let mut configs = ext.unwrap_or_default();
    names
        .into_iter()
        .map(|name| {
            let config = configs.remove(&name).and_then(|val| {
                // An empty sub-table means "no config needed".
                match &val {
                    toml::Value::Table(t) if t.is_empty() => None,
                    _ => serde_json::to_value(&val).ok(),
                }
            });
            ExtensionEntry { name, config }
        })
        .collect()
}

#[derive(Debug, serde::Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Network {
    pub name: Option<String>,
    pub rpc_url: Option<String>,
    pub network_passphrase: Option<String>,
    pub rpc_headers: Option<Vec<(String, String)>>,
    #[serde(skip_serializing_if = "is_false", default)]
    pub run_locally: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum AccountRepresentation {
    Simple(String),
    Detailed(Account),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Account {
    pub name: String,
    #[serde(default)]
    pub default: bool,
}

impl From<AccountRepresentation> for Account {
    fn from(rep: AccountRepresentation) -> Self {
        match rep {
            AccountRepresentation::Simple(name) => Account {
                name,
                default: false,
            },
            AccountRepresentation::Detailed(account) => account,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Contract {
    #[serde(default = "default_client", skip_serializing_if = "std::ops::Not::not")]
    pub client: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_deploy: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constructor_args: Option<String>,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            client: default_client(),
            after_deploy: None,
            id: None,
            constructor_args: None,
        }
    }
}

fn default_client() -> bool {
    true
}

impl Environment {
    pub fn get(
        workspace_root: &Path,
        scaffold_env: &ScaffoldEnv,
    ) -> Result<Option<Environment>, Error> {
        let env_toml = workspace_root.join(ENV_FILE);

        if !env_toml.exists() {
            return Ok(None);
        }

        let toml_str = std::fs::read_to_string(env_toml)?;
        let mut parsed_toml: Environments = toml::from_str(&toml_str)?;
        let current_env = parsed_toml.remove(scaffold_env.to_string().as_str());
        if current_env.is_none() {
            return Err(Error::NoSettingsForCurrentEnv(scaffold_env.to_string()));
        }
        Ok(current_env)
    }
}

impl From<&Network> for stellar_cli::config::network::Args {
    fn from(network: &Network) -> Self {
        stellar_cli::config::network::Args {
            network: network.name.clone(),
            rpc_url: network.rpc_url.clone(),
            network_passphrase: network.network_passphrase.clone(),
            rpc_headers: network
                .rpc_headers
                .iter()
                .flatten()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Parse a TOML string that contains exactly one environment keyed by
    /// `"development"` and return its [`Environment`].
    fn parse_dev(toml: &str) -> Environment {
        let mut envs: Environments = toml::from_str(toml).expect("invalid TOML");
        envs.remove("development").expect("missing development key")
    }

    // Minimal required fields for a valid Environment.
    const NETWORK_STUB: &str = r#"[development.network]
name = "testnet"
"#;

    #[test]
    fn extensions_with_config() {
        let toml = format!(
            r#"{NETWORK_STUB}
[development]
extensions = ["reporter", "indexer"]

[development.ext.reporter]
warn_size_kb = 128

[development.ext.indexer]
storage = "sqlite"
events = ["transfer", "mint"]
"#
        );

        let env = parse_dev(&toml);
        assert_eq!(env.extensions.len(), 2);

        let reporter = &env.extensions[0];
        assert_eq!(reporter.name, "reporter");
        assert_eq!(reporter.config, Some(json!({ "warn_size_kb": 128 })));

        let indexer = &env.extensions[1];
        assert_eq!(indexer.name, "indexer");
        assert_eq!(
            indexer.config,
            Some(json!({ "storage": "sqlite", "events": ["transfer", "mint"] }))
        );
    }

    #[test]
    fn extensions_without_config() {
        let toml = format!(
            r#"{NETWORK_STUB}
[development]
extensions = ["reporter", "indexer"]
"#
        );

        let env = parse_dev(&toml);
        assert_eq!(env.extensions.len(), 2);

        assert_eq!(env.extensions[0].name, "reporter");
        assert!(env.extensions[0].config.is_none());

        assert_eq!(env.extensions[1].name, "indexer");
        assert!(env.extensions[1].config.is_none());
    }

    #[test]
    fn extensions_empty_array() {
        let toml = format!(
            r"{NETWORK_STUB}
[development]
extensions = []
"
        );

        let env = parse_dev(&toml);
        assert!(env.extensions.is_empty());
    }

    #[test]
    fn extensions_absent() {
        let env = parse_dev(NETWORK_STUB);
        assert!(env.extensions.is_empty());
    }

    #[test]
    fn extensions_empty_ext_table() {
        // An ext sub-table with no keys → config should be None (not Some({})).
        let toml = format!(
            r#"{NETWORK_STUB}
[development]
extensions = ["linter"]

[development.ext.linter]
"#
        );

        let env = parse_dev(&toml);
        assert_eq!(env.extensions.len(), 1);
        assert_eq!(env.extensions[0].name, "linter");
        assert!(env.extensions[0].config.is_none());
    }

    #[test]
    fn extensions_ext_table_without_listing_is_ignored() {
        // An ext config for an extension not in the extensions array is ignored.
        let toml = format!(
            r#"{NETWORK_STUB}
[development]
extensions = ["reporter"]

[development.ext.reporter]
warn_size_kb = 128

[development.ext.unlisted]
some_key = "value"
"#
        );

        let env = parse_dev(&toml);
        assert_eq!(env.extensions.len(), 1);
        assert_eq!(env.extensions[0].name, "reporter");
    }
}
