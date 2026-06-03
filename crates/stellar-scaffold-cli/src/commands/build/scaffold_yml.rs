use std::path::Path;

/// Name of the scaffold configuration file.
pub const CONFIG_FILE: &str = "scaffold.yml";

/// The only `version:` value this CLI accepts in `scaffold.yml`.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(
        "scaffold.yml is missing or has no 'version' field. \
         If migrating from environments.toml, rename it to scaffold.yml and add 'version: {CURRENT_SCHEMA_VERSION}' at the top. \
         See https://github.com/theahaco/scaffold-stellar/blob/main/CHANGELOG.md for details."
    )]
    MissingVersion,
    #[error(
        "scaffold.yml uses schema version {found}, but this CLI supports version {CURRENT_SCHEMA_VERSION}. \
         See https://github.com/theahaco/scaffold-stellar/blob/main/CHANGELOG.md for migration instructions."
    )]
    UnsupportedVersion { found: u32 },
}

/// Configurable directory paths read from the `config:` section of `scaffold.yml`.
///
/// All fields default to the conventional locations used by the scaffold template.
///
/// Example `scaffold.yml`:
/// ```yaml
/// config:
///   contracts_dir: contracts
///   bindings_dir: bindings
///   clients_dir: core/clients
/// ```
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct ScaffoldConfig {
    /// Directory containing Rust/Soroban contract source (default: `"contracts"`).
    pub contracts_dir: std::path::PathBuf,
    /// Directory where generated Contract Bindings (TypeScript npm packages) are
    /// written (default: `"bindings"`). CLI-owned; kept separate from authored
    /// workspace packages so generated output is never overwritten or GC'd by hand.
    pub bindings_dir: std::path::PathBuf,
    /// Directory where the shared per-contract Contract Clients are generated
    /// (default: `"core/clients"`). One set shared by every template; imported
    /// by app code. See ADR 0009.
    pub clients_dir: std::path::PathBuf,
}

impl Default for ScaffoldConfig {
    fn default() -> Self {
        Self {
            contracts_dir: "contracts".into(),
            bindings_dir: "bindings".into(),
            clients_dir: "core/clients".into(),
        }
    }
}

/// Top-level structure of `scaffold.yml`.
#[derive(Debug, serde::Deserialize, Default)]
struct ScaffoldFile {
    version: Option<u32>,
    #[serde(default)]
    config: ScaffoldConfig,
}

impl ScaffoldConfig {
    /// Read `scaffold.yml` from `workspace_root` and return the `config:` section.
    ///
    /// Returns defaults if the file is absent, unreadable, or has no `config:` key.
    /// This function reads the file fresh on every call — it is not cached.
    pub fn get(workspace_root: &Path) -> ScaffoldConfig {
        let path = workspace_root.join(CONFIG_FILE);
        let Ok(contents) = std::fs::read_to_string(path) else {
            return ScaffoldConfig::default();
        };
        let Ok(file) = serde_yaml::from_str::<ScaffoldFile>(&contents) else {
            return ScaffoldConfig::default();
        };
        file.config
    }
}

/// Validate the `version:` field in `scaffold.yml`.
///
/// Returns `Ok(())` only if the file exists and its version matches
/// `CURRENT_SCHEMA_VERSION`. A missing file is treated as an outdated project
/// (no `scaffold.yml` means pre-versioning). A missing or unsupported version
/// field is also an error.
pub fn check_version(workspace_root: &Path) -> Result<(), Error> {
    let path = workspace_root.join(CONFIG_FILE);
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Err(Error::MissingVersion);
    };
    let Ok(file) = serde_yaml::from_str::<ScaffoldFile>(&contents) else {
        return Err(Error::MissingVersion);
    };
    match file.version {
        None => Err(Error::MissingVersion),
        Some(v) if v != CURRENT_SCHEMA_VERSION => Err(Error::UnsupportedVersion { found: v }),
        Some(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn defaults_when_file_absent() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = ScaffoldConfig::get(dir.path());
        assert_eq!(config.contracts_dir, PathBuf::from("contracts"));
        assert_eq!(config.bindings_dir, PathBuf::from("bindings"));
        assert_eq!(config.clients_dir, PathBuf::from("core/clients"));
    }

    #[test]
    fn reads_custom_config() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(CONFIG_FILE),
            "config:\n  contracts_dir: my_contracts\n  bindings_dir: my_packages\n  clients_dir: frontend/contracts\n",
        )
        .unwrap();
        let config = ScaffoldConfig::get(dir.path());
        assert_eq!(config.contracts_dir, PathBuf::from("my_contracts"));
        assert_eq!(config.bindings_dir, PathBuf::from("my_packages"));
        assert_eq!(config.clients_dir, PathBuf::from("frontend/contracts"));
    }

    #[test]
    fn defaults_when_no_config_section() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(CONFIG_FILE),
            "# just a comment, no config section\n",
        )
        .unwrap();
        let config = ScaffoldConfig::get(dir.path());
        assert_eq!(config.contracts_dir, PathBuf::from("contracts"));
        assert_eq!(config.bindings_dir, PathBuf::from("bindings"));
        assert_eq!(config.clients_dir, PathBuf::from("core/clients"));
    }

    #[test]
    fn check_version_err_when_file_absent() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(matches!(
            check_version(dir.path()),
            Err(Error::MissingVersion)
        ));
    }

    #[test]
    fn check_version_ok_for_current_version() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(CONFIG_FILE),
            format!("version: {CURRENT_SCHEMA_VERSION}\n"),
        )
        .unwrap();
        assert!(check_version(dir.path()).is_ok());
    }

    #[test]
    fn check_version_err_when_version_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(CONFIG_FILE),
            "config:\n  contracts_dir: contracts\n",
        )
        .unwrap();
        assert!(matches!(
            check_version(dir.path()),
            Err(Error::MissingVersion)
        ));
    }

    #[test]
    fn check_version_err_for_unsupported_version() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join(CONFIG_FILE), "version: 99\n").unwrap();
        assert!(matches!(
            check_version(dir.path()),
            Err(Error::UnsupportedVersion { found: 99 })
        ));
    }

    #[test]
    fn partial_config_uses_defaults_for_missing_fields() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(CONFIG_FILE),
            "config:\n  bindings_dir: my_packages\n",
        )
        .unwrap();
        let config = ScaffoldConfig::get(dir.path());
        assert_eq!(config.contracts_dir, PathBuf::from("contracts"));
        assert_eq!(config.bindings_dir, PathBuf::from("my_packages"));
        assert_eq!(config.clients_dir, PathBuf::from("core/clients"));
    }
}
