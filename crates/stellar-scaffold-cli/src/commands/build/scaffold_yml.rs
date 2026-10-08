use std::path::Path;

/// Name of the scaffold configuration file.
pub const CONFIG_FILE: &str = "scaffold.yml";

/// The `version:` of the original `config:`-only format. Version 2 is
/// `crate::config::SCHEMA_VERSION`; both are accepted.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(
        "scaffold.yml is missing or has no 'version' field. \
         If migrating from environments.toml, rename it to scaffold.yml and add 'version: {CURRENT_SCHEMA_VERSION}' at the top. \
         See https://github.com/stellar-scaffold/cli/blob/main/CHANGELOG.md for details."
    )]
    MissingVersion,
    #[error(
        "scaffold.yml uses schema version {found}, but this CLI supports versions {CURRENT_SCHEMA_VERSION} and {v2}. \
         See https://github.com/stellar-scaffold/cli/blob/main/CHANGELOG.md for migration instructions.",
        v2 = crate::config::SCHEMA_VERSION
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
///   clients_dir: app-lib/clients
/// ```
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct ScaffoldConfig {
    /// Directory containing Rust/Soroban contract source (default: `"contracts"`).
    pub contracts_dir: std::path::PathBuf,
    /// Directory for the generated TypeScript contract layer the app imports
    /// (default: `"app-lib/clients"`). Holds both the generated Contract Binding
    /// npm packages (one per contract, in `clients_dir/<name>/`) and the single
    /// flattened Contract Clients `index.ts` (`clients_dir/index.ts`), imported
    /// by app code as `@stellar-scaffold/app-lib/clients`. CLI-owned. (The
    /// former separate `bindings_dir` was folded into this one directory.)
    pub clients_dir: std::path::PathBuf,
    /// Package manager that installs and builds the generated clients. `None`
    /// when unset; callers fall back to npm.
    pub package_manager: Option<crate::commands::PackageManager>,
}

impl Default for ScaffoldConfig {
    fn default() -> Self {
        Self {
            contracts_dir: "contracts".into(),
            clients_dir: "app-lib/clients".into(),
            package_manager: None,
        }
    }
}

/// Top-level structure of `scaffold.yml`. Version 1 keeps directories under
/// `config:`; version 2 keeps them under `project:`.
#[derive(Debug, serde::Deserialize, Default)]
struct ScaffoldFile {
    version: Option<u32>,
    #[serde(default)]
    config: ScaffoldConfig,
    #[serde(default)]
    project: Option<ProjectDirs>,
}

/// The directory keys of a version 2 `project:` section.
#[derive(Debug, serde::Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
struct ProjectDirs {
    contracts_dir: Option<std::path::PathBuf>,
    clients_dir: Option<std::path::PathBuf>,
    package_manager: Option<crate::commands::PackageManager>,
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
        match file.project {
            Some(project) if file.version == Some(crate::config::SCHEMA_VERSION) => {
                let defaults = ScaffoldConfig::default();
                ScaffoldConfig {
                    contracts_dir: project.contracts_dir.unwrap_or(defaults.contracts_dir),
                    clients_dir: project.clients_dir.unwrap_or(defaults.clients_dir),
                    package_manager: project.package_manager,
                }
            }
            _ => file.config,
        }
    }
}

/// Validate the `version:` field in `scaffold.yml`.
///
/// Returns `Ok(())` only if the file exists and its version is
/// `CURRENT_SCHEMA_VERSION` or version 2. A missing file is treated as an outdated project
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
        Some(v) if v != CURRENT_SCHEMA_VERSION && v != crate::config::SCHEMA_VERSION => {
            Err(Error::UnsupportedVersion { found: v })
        }
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
        assert_eq!(config.clients_dir, PathBuf::from("app-lib/clients"));
    }

    #[test]
    fn reads_custom_config() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(CONFIG_FILE),
            "config:\n  contracts_dir: my_contracts\n  clients_dir: frontend/contracts\n",
        )
        .unwrap();
        let config = ScaffoldConfig::get(dir.path());
        assert_eq!(config.contracts_dir, PathBuf::from("my_contracts"));
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
        assert_eq!(config.clients_dir, PathBuf::from("app-lib/clients"));
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
    fn reads_version_2_project_dirs() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(CONFIG_FILE),
            "version: 2\nproject:\n  clients-dir: src/contracts\nnetworks: {}\n",
        )
        .unwrap();
        let config = ScaffoldConfig::get(dir.path());
        assert_eq!(config.contracts_dir, PathBuf::from("contracts"));
        assert_eq!(config.clients_dir, PathBuf::from("src/contracts"));
        assert!(check_version(dir.path()).is_ok());
    }

    #[test]
    fn partial_config_uses_defaults_for_missing_fields() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(CONFIG_FILE),
            "config:\n  clients_dir: my_clients\n",
        )
        .unwrap();
        let config = ScaffoldConfig::get(dir.path());
        assert_eq!(config.contracts_dir, PathBuf::from("contracts"));
        assert_eq!(config.clients_dir, PathBuf::from("my_clients"));
    }
}
