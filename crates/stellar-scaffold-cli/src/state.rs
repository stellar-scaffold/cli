//! `.scaffold/state.json`: the contracts `build` deployed from this checkout
//! of a project, so later builds upgrade or reuse them instead of guessing
//! from stellar-cli's machine-wide contract aliases, and `clean` knows what
//! to remove. Machine-local and regenerable: the directory ignores itself.

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

/// Directory at the workspace root holding Scaffold's machine-local files.
pub const DIR: &str = ".scaffold";

/// File within [`DIR`] recording deployments.
pub const FILE: &str = "state.json";

/// Format version of [`FILE`]. Independent of `scaffold.yml`'s version: the
/// file is generated, so a version this Scaffold can't read is discarded
/// rather than migrated.
pub const VERSION: u32 = 1;

/// Written into [`DIR`] so its contents stay out of git even in projects
/// whose `.gitignore` doesn't list it.
const GITIGNORE: &str = "# Machine-local Scaffold state, created by `stellar scaffold build`\n*\n";

/// Longest name stellar-cli accepts for an alias.
const MAX_ALIAS_LEN: usize = 250;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("version {0} is not supported; this Scaffold reads version {VERSION}")]
    UnsupportedVersion(u32),
}

/// Everything recorded for a checkout, keyed by network.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct State {
    pub version: u32,
    #[serde(default)]
    pub networks: BTreeMap<String, NetworkState>,
}

/// Deployments on one network, keyed by its name in `scaffold.yml` (or the
/// environment name for environments.toml projects). The passphrase detects
/// a name that now points at a different chain.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct NetworkState {
    pub network_passphrase: String,
    #[serde(default)]
    pub contracts: BTreeMap<String, ContractState>,
}

/// One deployed contract, keyed by its name under `contracts:`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct ContractState {
    pub contract_id: String,
    /// The stellar-cli alias Scaffold saved for this contract, if any, so
    /// `clean` removes exactly that alias even after the directory is renamed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

/// Read before the full parse, so an unsupported version is reported as
/// such instead of as whatever field it happens to break.
#[derive(Deserialize)]
struct Header {
    version: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            version: VERSION,
            networks: BTreeMap::new(),
        }
    }
}

impl State {
    /// Where the state file lives for the workspace at `root`.
    pub fn path(root: &Path) -> PathBuf {
        root.join(DIR).join(FILE)
    }

    /// The recorded state for `root`; empty when there is no file yet.
    pub fn load(root: &Path) -> Result<Self, Error> {
        let contents = match fs::read_to_string(Self::path(root)) {
            Ok(contents) => contents,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.into()),
        };
        Self::parse(&contents)
    }

    fn parse(contents: &str) -> Result<Self, Error> {
        let Header { version } = serde_json::from_str(contents)?;
        if version != VERSION {
            return Err(Error::UnsupportedVersion(version));
        }
        Ok(serde_json::from_str(contents)?)
    }

    /// Write the state for `root`, creating [`DIR`] and its `.gitignore`
    /// first. Written to a temporary file and renamed, so an interrupted
    /// build never leaves a half-written file behind.
    pub fn save(&self, root: &Path) -> Result<(), Error> {
        let dir = root.join(DIR);
        fs::create_dir_all(&dir)?;
        let gitignore = dir.join(".gitignore");
        if !gitignore.exists() {
            fs::write(gitignore, GITIGNORE)?;
        }
        let path = Self::path(root);
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, format!("{}\n", serde_json::to_string_pretty(self)?))?;
        fs::rename(tmp, path)?;
        Ok(())
    }

    /// Delete the state file for `root`, returning whether there was one.
    pub fn remove(root: &Path) -> io::Result<bool> {
        match fs::remove_file(Self::path(root)) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// The contract recorded as `name` on `network`, unless that network
    /// was recorded with a different passphrase.
    pub fn contract(&self, network: &str, passphrase: &str, name: &str) -> Option<&ContractState> {
        self.networks
            .get(network)
            .filter(|n| n.network_passphrase == passphrase)?
            .contracts
            .get(name)
    }

    /// Record `contract` as `name` on `network`. Entries recorded under a
    /// different passphrase belong to another chain and are dropped.
    pub fn record(&mut self, network: &str, passphrase: &str, name: &str, contract: ContractState) {
        let entry = self
            .networks
            .entry(network.to_string())
            .or_insert_with(|| NetworkState {
                network_passphrase: passphrase.to_string(),
                contracts: BTreeMap::new(),
            });
        if entry.network_passphrase != passphrase {
            entry.network_passphrase = passphrase.to_string();
            entry.contracts.clear();
        }
        entry.contracts.insert(name.to_string(), contract);
    }

    /// Every recorded alias, as `(passphrase, alias, contract_id)`.
    pub fn aliases(&self) -> impl Iterator<Item = (&str, &str, &str)> {
        self.networks.values().flat_map(|network| {
            network.contracts.values().filter_map(|contract| {
                Some((
                    network.network_passphrase.as_str(),
                    contract.alias.as_deref()?,
                    contract.contract_id.as_str(),
                ))
            })
        })
    }
}

/// The stellar-cli alias for contract `name` in the workspace at `root`:
/// the workspace directory's name, then `name`, e.g. `my-app-guess_the_number`.
/// Separate checkouts, such as git worktrees, get separate aliases. Characters
/// stellar-cli doesn't allow in names become `-`. `None` when there's no
/// usable directory name or the result is too long.
pub fn alias_name(root: &Path, name: &str) -> Option<String> {
    let dir = root.file_name()?.to_string_lossy();
    let prefix: String = dir
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let prefix = prefix.trim_matches('-');
    if prefix.is_empty() {
        return None;
    }
    let alias = format!("{prefix}-{name}");
    (alias.len() <= MAX_ALIAS_LEN).then_some(alias)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCAL: &str = "Standalone Network ; February 2017";
    const TESTNET: &str = "Test SDF Network ; September 2015";

    fn deployed(id: &str) -> ContractState {
        ContractState {
            contract_id: id.to_string(),
            alias: Some(format!("app-{id}")),
        }
    }

    #[test]
    fn missing_file_is_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(State::load(dir.path()).unwrap(), State::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = State::default();
        state.record("local", LOCAL, "guess_the_number", deployed("C1"));
        state.save(dir.path()).unwrap();
        assert_eq!(State::load(dir.path()).unwrap(), state);
    }

    #[test]
    fn saved_file_uses_the_documented_shape() {
        let mut state = State::default();
        state.record("local", LOCAL, "guess_the_number", deployed("C1"));
        let json: serde_json::Value = serde_json::to_value(&state).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "version": 1,
                "networks": {
                    "local": {
                        "network-passphrase": LOCAL,
                        "contracts": {
                            "guess_the_number": { "contract-id": "C1", "alias": "app-C1" }
                        }
                    }
                }
            })
        );
    }

    #[test]
    fn save_ignores_its_own_directory() {
        let dir = tempfile::tempdir().unwrap();
        State::default().save(dir.path()).unwrap();
        let gitignore = fs::read_to_string(dir.path().join(DIR).join(".gitignore")).unwrap();
        assert!(gitignore.lines().any(|line| line == "*"));
    }

    #[test]
    fn unsupported_version_is_an_error() {
        let err = State::parse(r#"{ "version": 2, "networks": { "local": 7 } }"#).unwrap_err();
        assert!(matches!(err, Error::UnsupportedVersion(2)));
    }

    #[test]
    fn contract_on_a_redefined_network_is_not_found() {
        let mut state = State::default();
        state.record("local", LOCAL, "counter", deployed("C1"));
        assert!(state.contract("local", LOCAL, "counter").is_some());
        // `local` now names a different chain
        assert!(state.contract("local", TESTNET, "counter").is_none());
    }

    #[test]
    fn recording_on_a_redefined_network_drops_its_old_entries() {
        let mut state = State::default();
        state.record("dev", LOCAL, "counter", deployed("C1"));
        state.record("dev", TESTNET, "token", deployed("C2"));
        let dev = &state.networks["dev"];
        assert_eq!(dev.network_passphrase, TESTNET);
        assert_eq!(dev.contracts.keys().collect::<Vec<_>>(), ["token"]);
    }

    #[test]
    fn aliases_skip_contracts_without_one() {
        let mut state = State::default();
        state.record("local", LOCAL, "counter", deployed("C1"));
        state.record(
            "local",
            LOCAL,
            "token",
            ContractState {
                contract_id: "C2".into(),
                alias: None,
            },
        );
        assert_eq!(
            state.aliases().collect::<Vec<_>>(),
            [(LOCAL, "app-C1", "C1")]
        );
    }

    #[test]
    fn remove_reports_whether_there_was_a_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!State::remove(dir.path()).unwrap());
        State::default().save(dir.path()).unwrap();
        assert!(State::remove(dir.path()).unwrap());
        assert!(!State::path(dir.path()).exists());
    }

    #[test]
    fn alias_is_prefixed_with_the_directory_name() {
        assert_eq!(
            alias_name(Path::new("/code/my-app"), "guess_the_number").as_deref(),
            Some("my-app-guess_the_number")
        );
        // characters stellar-cli rejects in names
        assert_eq!(
            alias_name(Path::new("/code/My App.v2"), "token").as_deref(),
            Some("My-App-v2-token")
        );
        assert_eq!(alias_name(Path::new("/"), "token"), None);
        assert_eq!(alias_name(Path::new("/code/..."), "token"), None);
        assert_eq!(alias_name(Path::new(&"a".repeat(250)), "token"), None);
    }
}
