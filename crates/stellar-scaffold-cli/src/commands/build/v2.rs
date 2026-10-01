//! Build input from a version 2 `scaffold.yml`.
//!
//! Resolves the config for one network and translates it into the
//! `Environment` the clients pipeline already consumes, so projects on either
//! config format share one deploy path until `environments.toml` is retired.

use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use stellar_cli::print::Print;

use super::env_toml::{self, Account, Environment, ExtensionEntry, ResolvedDeploy};
use crate::config::{self, Code, cli_args, interpolate, resolve, source::SourceType};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Rendered diagnostics from `scaffold.yml`.
    #[error("{0}")]
    Invalid(String),
    #[error("⛔ ️{0}")]
    Resolve(String),
    #[error("⛔ ️contract `{name}` uses {feature}, which `build` does not support yet")]
    Unsupported { name: String, feature: String },
}

/// Whether the project at `root` uses a version 2 `scaffold.yml`.
pub fn is_v2(root: &Path) -> bool {
    config::declared_version(root) == Some(u64::from(config::SCHEMA_VERSION))
}

/// The network to build for, and where the choice came from: `--network` or
/// `STELLAR_NETWORK` (passed in as `flag`), else stellar-cli's default from
/// `stellar network use`, else `local`.
pub fn select_network(flag: Option<&str>) -> (String, &'static str) {
    let stellar_default = stellar_cli::config::Config::new()
        .ok()
        .and_then(|c| c.defaults.network);
    pick_network(flag, stellar_default)
}

fn pick_network(flag: Option<&str>, stellar_default: Option<String>) -> (String, &'static str) {
    match (flag, stellar_default) {
        (Some(name), _) => (name.to_string(), "--network or STELLAR_NETWORK"),
        (None, Some(name)) => (name, "`stellar network use`"),
        (None, None) => (config::DEFAULT_NETWORK.to_string(), "default"),
    }
}

/// Validate `scaffold.yml` in `root` without selecting a network. Warnings
/// are printed; errors are returned rendered.
pub fn validate(
    root: &Path,
    crates: Option<&[(String, PathBuf)]>,
    printer: &Print,
) -> Result<config::Config, Error> {
    loaded_config(config::load(root, crates, None), printer)
}

/// Validate `scaffold.yml` for `network_name` and translate it into the
/// `Environment` the clients pipeline consumes. Every contract must list the
/// network.
pub fn plan(
    root: &Path,
    crates: Option<&[(String, PathBuf)]>,
    network_name: &str,
    printer: &Print,
) -> Result<Environment, Error> {
    let config = loaded_config(config::load(root, crates, Some(network_name)), printer)?;
    translate(&config, network_name, &|name| std::env::var(name).ok())
}

fn loaded_config(mut loaded: config::Loaded, printer: &Print) -> Result<config::Config, Error> {
    // Without cargo metadata the crate check is skipped; not worth a warning
    // on every build.
    loaded
        .diagnostics
        .retain(|d| d.code != Code::WorkspaceUnavailable);
    if loaded.has_errors() {
        loaded.diagnostics.retain(config::Diagnostic::is_error);
        return Err(Error::Invalid(loaded.render()));
    }
    if !loaded.diagnostics.is_empty() {
        printer.warnln(format!("{} has warnings:", config::CONFIG_FILE));
        eprint!("{}", loaded.render());
    }
    Ok(loaded
        .config
        .expect("a config without errors is always present"))
}

fn translate(
    config: &config::Config,
    network_name: &str,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Environment, Error> {
    let mut resolver = resolve::Resolver::new(config);
    let net = resolver
        .network(network_name)
        .ok_or_else(|| Error::Resolve(format!("network `{network_name}` could not be resolved")))?;

    let net_ctx = interpolate::Context {
        env,
        network: None,
        mode: interpolate::Mode::Final,
    };
    let resolve =
        |t: &resolve::Template| interpolate::resolve(&t.segments, &net_ctx).map_err(Error::Resolve);
    let rpc_url = resolve(net.rpc_url.as_ref().expect("validated"))?;
    let network_passphrase = resolve(net.passphrase.as_ref().expect("validated"))?;
    let rpc_headers = net
        .rpc_headers
        .iter()
        .map(|(k, t)| Ok((k.clone(), resolve(t)?)))
        .collect::<Result<Vec<_>, Error>>()?;

    let args_ctx = interpolate::Context {
        env,
        network: Some((&net.name, &rpc_url, &network_passphrase)),
        mode: interpolate::Mode::Final,
    };
    let mut contracts = IndexMap::new();
    for name in config.contracts.keys() {
        let Some(c) = resolver.contract(name, network_name) else {
            continue;
        };
        contracts.insert(name.into(), contract(&c, &args_ctx)?);
    }

    let default_account = net.default_account().map(str::to_string);
    Ok(Environment {
        accounts: Some(
            net.accounts
                .iter()
                .map(|name| Account {
                    default: Some(name) == default_account.as_ref(),
                    name: name.clone(),
                })
                .collect(),
        ),
        network: env_toml::Network {
            name: Some(net.name.clone()),
            rpc_url: Some(rpc_url),
            network_passphrase: Some(network_passphrase),
            rpc_headers: Some(rpc_headers),
            run_locally: net.start_container(),
        },
        contracts: Some(contracts),
        extensions: extension_entries(config),
        from_scaffold_yml: true,
    })
}

/// Extensions and the hook `env` label for a version 2 project that isn't
/// building clients: validates `scaffold.yml` without resolving a network.
pub fn hook_inputs(
    root: &Path,
    crates: Option<&[(String, PathBuf)]>,
    network_flag: Option<&str>,
    printer: &Print,
) -> Result<(Vec<ExtensionEntry>, String), Error> {
    let config = validate(root, crates, printer)?;
    let (network, _) = select_network(network_flag);
    Ok((extension_entries(&config), network))
}

/// Extensions in run order, with their config as JSON.
pub fn extension_entries(config: &config::Config) -> Vec<ExtensionEntry> {
    config
        .extensions
        .iter()
        .map(|(name, value)| ExtensionEntry {
            name: name.value.clone(),
            config: value.as_ref().map(config::schema::Value::to_json),
        })
        .collect()
}

/// A resolved contract as a pipeline entry: `workspace` sources deploy their
/// crate; `contract` sources reuse the pinned-`id` path.
fn contract(
    c: &resolve::Contract,
    ctx: &interpolate::Context,
) -> Result<env_toml::Contract, Error> {
    let unsupported = |feature: String| Error::Unsupported {
        name: c.name.clone(),
        feature,
    };
    if c.from_network.is_some() {
        return Err(unsupported("`from-network`".to_string()));
    }
    if c.after_deploy_script.is_some() {
        return Err(unsupported("`after-deploy-script`".to_string()));
    }
    let mut entry = env_toml::Contract::default();
    match c.ty {
        SourceType::Workspace => {
            entry.resolved = Some(ResolvedDeploy {
                crate_name: Some(c.source.clone()),
                signer: c.signer.clone(),
                constructor_args: cli_args::constructor_args(&c.args, ctx)
                    .map_err(Error::Resolve)?,
                after_deploy: c.after_deploy.clone(),
            });
        }
        SourceType::Contract => {
            entry.id = Some(c.source.clone());
            entry.resolved = Some(ResolvedDeploy::default());
        }
        other => return Err(unsupported(format!("`type: {}`", other.as_str()))),
    }
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTRACT: &str = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";

    fn translate_yaml(yaml: &str, network: &str) -> Result<Environment, Error> {
        let config = config::schema::parse(yaml).unwrap();
        let env = |name: &str| (name == "TOKEN").then(|| "t".to_string());
        translate(&config, network, &env)
    }

    #[test]
    fn network_precedence() {
        assert_eq!(
            pick_network(Some("preview"), Some("testnet".into())).0,
            "preview"
        );
        assert_eq!(pick_network(None, Some("testnet".into())).0, "testnet");
        assert_eq!(pick_network(None, None).0, "local");
    }

    #[test]
    fn translates_network_accounts_and_contracts() {
        let yaml = format!(
            r#"version: 2
networks:
  local:
    accounts: [me, admin]
    default-account: admin
    rpc-headers: {{ Authorization: "Bearer ${{env.TOKEN}}" }}
extensions:
  reporter: {{ warn-size-kb: 128 }}
contracts:
  my-token:
    type: workspace
    source: fungible-token
    signer: me
    args: {{ owner: "${{account.admin}}", supply: 1000000000000000000000000 }}
    after-deploy: [init]
    networks: {{ local: }}
  dex:
    type: contract
    source: {CONTRACT}
    networks: {{ local: }}
"#
        );
        let env = translate_yaml(&yaml, "local").unwrap();
        assert_eq!(env.network.name.as_deref(), Some("local"));
        assert_eq!(
            env.network.rpc_url.as_deref(),
            Some("http://localhost:8000/rpc")
        );
        assert_eq!(
            env.network.rpc_headers,
            Some(vec![("Authorization".into(), "Bearer t".into())])
        );
        assert!(env.network.run_locally);

        assert!(env.from_scaffold_yml);
        let accounts = env.accounts.as_ref().unwrap();
        assert_eq!(
            accounts
                .iter()
                .filter(|a| a.default)
                .map(|a| &a.name)
                .collect::<Vec<_>>(),
            vec!["admin"]
        );
        assert_eq!(
            env.extensions[0].config,
            Some(serde_json::json!({ "warn-size-kb": 128 }))
        );

        let contracts = env.contracts.as_ref().unwrap();
        let token = contracts
            .get("my-token")
            .unwrap()
            .resolved
            .as_ref()
            .unwrap();
        assert_eq!(token.crate_name.as_deref(), Some("fungible-token"));
        assert_eq!(token.signer.as_deref(), Some("me"));
        assert_eq!(
            token.constructor_args,
            vec!["--owner", "admin", "--supply", "1000000000000000000000000"]
        );
        assert_eq!(token.after_deploy, vec!["init"]);
        assert_eq!(contracts.get("dex").unwrap().id.as_deref(), Some(CONTRACT));
    }

    #[test]
    fn unsupported_sources_are_errors() {
        let yaml = "version: 2\nnetworks:\n  local: {}\ncontracts:\n  r:\n    type: registry\n    source: thing@1.0.0\n    networks: { local: }\n";
        assert!(matches!(
            translate_yaml(yaml, "local"),
            Err(Error::Unsupported { .. })
        ));
    }

    #[test]
    fn missing_env_is_an_error() {
        let yaml = "version: 2\nnetworks:\n  local:\n    rpc-headers: { A: \"${env.NOPE}\" }\n";
        assert!(matches!(
            translate_yaml(yaml, "local"),
            Err(Error::Resolve(_))
        ));
    }
}
