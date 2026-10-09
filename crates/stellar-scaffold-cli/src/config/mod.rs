//! `scaffold.yml` schema
//!
//! - `schema.rs`      :: typed config, deserialized with serde-saphyr
//! - `diagnostic.rs`  :: coded diagnostics and their rendering
//! - `lint.rs`        :: checks on individual values
//! - `source.rs`      :: contract `type` kinds and `source` validation
//! - `interpolate.rs` :: `${namespace.path}` grammar
//! - `cli_args.rs`    :: constructor `args` as stellar-cli arguments
//! - `resolve.rs`     :: `extends` and per-network merging, cross-reference rules
//! - `edit.rs`        :: line-based edits that keep comments and layout
//!
//! [`load`] runs the whole pipeline and never fails: every problem is a
//! [`Diagnostic`]. Callers decide what to do with errors.
//!
//! Projects still configured by the deprecated `environments.toml` are
//! recognized by that file alone; from their `scaffold.yml`, only a version 1
//! `config:` section's directories are read.

pub mod cli_args;
pub mod diagnostic;
pub mod edit;
pub mod interpolate;
pub mod lint;
pub mod resolve;
pub mod schema;
pub mod source;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

pub use diagnostic::{Code, Diagnostic, Severity};
pub use schema::Config;

/// Name of the config file at the project root.
pub const CONFIG_FILE: &str = "scaffold.yml";

/// Schema version this module reads.
pub const SCHEMA_VERSION: u32 = 2;

#[derive(thiserror::Error, Debug)]
pub enum VersionError {
    #[error(
        "scaffold.yml is missing or has no 'version' field. \
         Add 'version: {SCHEMA_VERSION}' at the top. \
         See https://github.com/stellar-scaffold/cli/blob/main/CHANGELOG.md for details."
    )]
    MissingVersion,
    #[error(
        "scaffold.yml uses schema version {found}, but this CLI supports version {SCHEMA_VERSION}. \
         See https://github.com/stellar-scaffold/cli/blob/main/CHANGELOG.md for migration instructions."
    )]
    UnsupportedVersion { found: u64 },
}

/// Environment variable selecting the network when `--network` is absent.
/// Shared with stellar-cli, which also sets it from `stellar network use`.
pub const NETWORK_ENV: &str = "STELLAR_NETWORK";

/// Network used when neither `--network`, [`NETWORK_ENV`], nor stellar-cli's
/// default picks one.
pub const DEFAULT_NETWORK: &str = "local";

/// Names of networks and accounts: ASCII letters, digits, `-`, `_`, starting
/// with a letter or digit.
pub fn is_valid_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The result of loading `scaffold.yml`.
pub struct Loaded {
    /// Path of the file, for rendering diagnostics.
    pub path: PathBuf,
    /// Source text, for rendering diagnostics.
    pub source: String,
    /// The config, if the file parsed far enough to produce one.
    pub config: Option<Config>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Loaded {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// Render every diagnostic, each followed by a blank line.
    pub fn render(&self) -> String {
        let file = self.path.file_name().map_or_else(
            || CONFIG_FILE.to_string(),
            |f| f.to_string_lossy().into_owned(),
        );
        self.diagnostics
            .iter()
            .map(|d| d.render(&file, &self.source) + "\n")
            .collect()
    }
}

/// Where a project lives and which contract crates it has.
pub struct Workspace {
    pub root: PathBuf,
    /// `(package name, directory)` of every `cdylib` crate, or `None` if
    /// cargo metadata could not be read.
    pub crates: Option<Vec<(String, PathBuf)>>,
}

impl Workspace {
    /// Locate the cargo workspace from `manifest_path` or the current
    /// directory. Without cargo metadata, the manifest's directory (or the
    /// current directory) is the root and crate checks are skipped.
    pub fn discover(manifest_path: Option<&Path>) -> Self {
        let mut cmd = cargo_metadata::MetadataCommand::new();
        cmd.no_deps();
        if let Some(path) = manifest_path {
            cmd.manifest_path(path);
        }
        let Ok(metadata) = cmd.exec() else {
            let root = manifest_path
                .and_then(Path::parent)
                .map(Path::to_path_buf)
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_default();
            return Self { root, crates: None };
        };
        Self {
            crates: Some(cdylib_crates(&metadata)),
            root: metadata.workspace_root.into_std_path_buf(),
        }
    }

    /// Load and validate this workspace's `scaffold.yml`. See [`load`].
    pub fn load(&self, selected: Option<&str>) -> Loaded {
        load(&self.root, self.crates.as_deref(), selected)
    }
}

/// `(package name, directory)` of every `cdylib` crate in the workspace.
pub fn cdylib_crates(metadata: &cargo_metadata::Metadata) -> Vec<(String, PathBuf)> {
    metadata
        .packages
        .iter()
        .filter(|p| {
            p.targets
                .iter()
                .any(|t| t.crate_types.iter().any(|c| c == "cdylib"))
        })
        .filter_map(|p| {
            let dir = p.manifest_path.parent()?.as_std_path().to_path_buf();
            Some((p.name.clone(), dir))
        })
        .collect()
}

/// Load and validate `scaffold.yml` in `root`.
///
/// `crates` enables the check that `workspace` sources name real crates.
/// `selected` additionally requires every contract to have an entry for that
/// network.
pub fn load(root: &Path, crates: Option<&[(String, PathBuf)]>, selected: Option<&str>) -> Loaded {
    let path = root.join(CONFIG_FILE);
    let mut loaded = Loaded {
        path: path.clone(),
        source: String::new(),
        config: None,
        diagnostics: Vec::new(),
    };
    match std::fs::read_to_string(&path) {
        Ok(source) => loaded.source = source,
        Err(e) => {
            loaded.diagnostics.push(
                Diagnostic::error(
                    Code::SchemaVersion,
                    format!("cannot read {}: {e}", path.display()),
                )
                .help(format!(
                    "create {CONFIG_FILE} starting with `version: {SCHEMA_VERSION}`"
                )),
            );
            return loaded;
        }
    }
    let config = match schema::parse(&loaded.source) {
        Ok(config) => config,
        Err(d) => {
            loaded.diagnostics.push(d);
            return loaded;
        }
    };
    if crates.is_none() {
        loaded.diagnostics.push(
            Diagnostic::warning(Code::WorkspaceUnavailable, "could not read the cargo workspace; skipped checking that `workspace` crates exist")
                .help("run from inside the project, or pass --manifest-path"),
        );
    }
    let env = lint::Env { root, crates };
    loaded.diagnostics.extend(lint::lint(&config, &env));
    let mut resolver = resolve::Resolver::new(&config);
    resolver.validate_all(selected);
    loaded.diagnostics.append(&mut resolver.diagnostics);
    // Errors first, then in file order, for every output format.
    loaded
        .diagnostics
        .sort_by_key(|d| (d.severity, d.span.map(|s| (s.line, s.col))));
    loaded.config = Some(config);
    loaded
}

/// The fully resolved config for one network, as printed by `config show`.
/// Keys keep their insertion order, which `config show` prints as YAML or JSON.
///
/// `env` and `network` references are substituted; `account` references stay
/// symbolic because addresses may not exist until `build` creates the keys.
pub fn resolved_view(
    config: &Config,
    network: &str,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<serde_json::Value, Vec<Diagnostic>> {
    use serde_json::{Map, Value};

    if !config.networks.contains_key(network) {
        let mut d = Diagnostic::error(
            Code::UnknownNetwork,
            format!("network `{network}` is not declared under `networks:`"),
        );
        if let Some(s) = diagnostic::suggest(network, config.networks.keys()) {
            d = d.help(format!("did you mean `{s}`?"));
        }
        return Err(vec![d]);
    }
    let mut resolver = resolve::Resolver::new(config);
    let Some(net) = resolver.network(network) else {
        return Err(vec![Diagnostic::error(
            Code::UnknownNetwork,
            format!("network `{network}` could not be resolved"),
        )]);
    };
    let mut errors = Vec::new();
    let mut interp = |t: &resolve::Template, ctx: &interpolate::Context| {
        interpolate::resolve(&t.segments, ctx).unwrap_or_else(|e| {
            errors.push(Diagnostic::error(Code::Interpolation, e));
            t.raw.clone()
        })
    };
    let net_ctx = interpolate::Context {
        env,
        network: None,
        mode: interpolate::Mode::Display,
    };
    let rpc_url = net
        .rpc_url
        .as_ref()
        .map(|t| interp(t, &net_ctx))
        .unwrap_or_default();
    let passphrase = net
        .passphrase
        .as_ref()
        .map(|t| interp(t, &net_ctx))
        .unwrap_or_default();

    let mut n = Map::new();
    n.insert("name".into(), net.name.clone().into());
    if !net.extends.is_empty() {
        n.insert("extends".into(), net.extends.clone().into());
    }
    n.insert("rpc-url".into(), rpc_url.clone().into());
    n.insert("network-passphrase".into(), passphrase.clone().into());
    if !net.rpc_headers.is_empty() {
        let headers: Map<String, Value> = net
            .rpc_headers
            .iter()
            .map(|(k, t)| (k.clone(), interp(t, &net_ctx).into()))
            .collect();
        n.insert("rpc-headers".into(), headers.into());
    }
    n.insert("accounts".into(), net.accounts.clone().into());
    if let Some(d) = net.default_account() {
        n.insert("default-account".into(), d.into());
    }
    n.insert("start-container".into(), net.start_container().into());
    n.insert("allow-deploy".into(), net.allow_deploy().into());
    n.insert("allow-http".into(), net.allow_http().into());

    let args_ctx = interpolate::Context {
        mode: interpolate::Mode::Display,
        env,
        network: Some((&net.name, &rpc_url, &passphrase)),
    };
    let mut contracts = Map::new();
    for name in config.contracts.keys() {
        let Some(c) = resolver.contract(name, network) else {
            continue;
        };
        let view = contract_view(&c, &mut |s| interp(&resolve::Template::new(s), &args_ctx));
        contracts.insert(name.into(), view);
    }
    let mut errors_all: Vec<Diagnostic> = resolver
        .diagnostics
        .into_iter()
        .filter(Diagnostic::is_error)
        .collect();
    errors_all.append(&mut errors);
    if !errors_all.is_empty() {
        return Err(errors_all);
    }

    let mut out = Map::new();
    out.insert("network".into(), n.into());
    out.insert("contracts".into(), contracts.into());
    Ok(Value::Object(out))
}

fn contract_view(
    c: &resolve::Contract,
    resolve: &mut dyn FnMut(&str) -> String,
) -> serde_json::Value {
    use serde_json::{Map, Value};
    let mut m = Map::new();
    m.insert("type".into(), c.ty.as_str().into());
    m.insert("source".into(), c.source.clone().into());
    if let Some(f) = &c.from_network {
        m.insert("from-network".into(), f.clone().into());
    }
    m.insert("deploy".into(), c.deploys.into());
    m.insert("client".into(), c.client.into());
    if let Some(s) = &c.signer {
        m.insert("signer".into(), s.clone().into());
    }
    if !c.args.is_empty() {
        let args: Map<String, Value> = c
            .args
            .iter()
            .map(|(k, v)| (k.value.clone(), value_view(&v.value, resolve)))
            .collect();
        m.insert("args".into(), args.into());
    }
    if !c.after_deploy.is_empty() {
        m.insert("after-deploy".into(), c.after_deploy.clone().into());
    }
    if let Some(s) = &c.after_deploy_script {
        m.insert("after-deploy-script".into(), s.clone().into());
    }
    m.into()
}

/// Convert a value for display, interpolating strings. Integers that don't
/// fit 64 bits are shown as strings rather than lossy floats.
fn value_view(value: &schema::Value, resolve: &mut dyn FnMut(&str) -> String) -> serde_json::Value {
    use schema::Value as V;
    use serde_json::Value;
    match value {
        V::Null => Value::Null,
        V::Bool(b) => (*b).into(),
        V::Int(s) => s
            .parse::<i64>()
            .map(Value::from)
            .or_else(|_| s.parse::<u64>().map(Value::from))
            .unwrap_or_else(|_| s.clone().into()),
        V::Float(f) => (*f).into(),
        V::Str(s) => resolve(s).into(),
        V::Seq(items) => items
            .iter()
            .map(|i| value_view(&i.value, resolve))
            .collect::<Vec<_>>()
            .into(),
        V::Map(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.value.clone(), value_view(&v.value, resolve)))
                .collect(),
        ),
    }
}

/// The `version` declared by `scaffold.yml` in `root`, if any.
pub fn declared_version(root: &Path) -> Option<u64> {
    let source = std::fs::read_to_string(root.join(CONFIG_FILE)).ok()?;
    schema::declared_version(&source)
}

/// Check that `scaffold.yml` in `root` declares [`SCHEMA_VERSION`]. A project
/// configured by the deprecated `environments.toml` passes whatever its
/// `scaffold.yml` says, or without one.
pub fn check_version(root: &Path) -> Result<(), VersionError> {
    match declared_version(root) {
        Some(v) if v == u64::from(SCHEMA_VERSION) => Ok(()),
        _ if uses_env_toml(root) => Ok(()),
        None => Err(VersionError::MissingVersion),
        Some(found) => Err(VersionError::UnsupportedVersion { found }),
    }
}

/// Whether `root` has the deprecated `environments.toml`. A version 2
/// `scaffold.yml` takes precedence when both exist.
pub fn uses_env_toml(root: &Path) -> bool {
    root.join(crate::commands::build::env_toml::ENV_FILE)
        .is_file()
}

/// The `project:` section of `scaffold.yml` in `root`. Beside the deprecated
/// `environments.toml`, a version 1 file's `config:` directories are read
/// instead. Defaults when the file is absent or doesn't parse; commands that
/// need a valid file report that through [`load`].
pub fn project(root: &Path) -> schema::Project {
    let Ok(source) = std::fs::read_to_string(root.join(CONFIG_FILE)) else {
        return schema::Project::default();
    };
    if let Ok(config) = schema::parse(&source) {
        return config.project;
    }
    // A version 1 file only counts beside environments.toml, matching check_version
    if uses_env_toml(root) {
        let dirs = serde_saphyr::from_str::<LegacyFile>(&source)
            .map(|file| file.config)
            .unwrap_or_default();
        let defaults = schema::Project::default();
        return schema::Project {
            contracts_dir: dirs.contracts_dir.unwrap_or(defaults.contracts_dir),
            clients_dir: dirs.clients_dir.unwrap_or(defaults.clients_dir),
            package_manager: None,
        };
    }
    schema::Project::default()
}

/// A version 1 `scaffold.yml`, read only beside the deprecated
/// `environments.toml` so its custom directories keep working. Other keys,
/// including `version`, are ignored.
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct LegacyFile {
    config: LegacyDirs,
}

/// The directories a version 1 `config:` section may override.
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct LegacyDirs {
    contracts_dir: Option<PathBuf>,
    clients_dir: Option<PathBuf>,
}
