use std::{
    fs::read_to_string,
    io,
    path::Path,
    process::{Command, Output},
    str::FromStr,
};

use clap::{CommandFactory, FromArgMatches, Parser};
use regex::Regex;
use stellar_cli;

pub mod build;
pub mod clean;
pub mod ext;
pub mod generate;
pub mod init;
pub mod setup;
pub mod update_env;
pub mod upgrade;
pub mod version;
pub mod watch;

const ABOUT: &str = "Build smart contracts with frontend support";

#[derive(Parser, Debug)]
#[command(
    name = "stellar-scaffold",
    about = ABOUT,
    disable_help_subcommand = true,
    version = version::long()
)]
pub struct Root {
    #[clap(flatten)]
    pub global_args: stellar_cli::commands::global::Args,

    #[command(subcommand)]
    pub cmd: Cmd,
}

impl Root {
    pub fn new() -> Result<Self, clap::Error> {
        let mut matches = Self::command().get_matches();
        Self::from_arg_matches_mut(&mut matches)
    }

    pub fn from_arg_matches<I, T>(itr: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        Self::from_arg_matches_mut(&mut Self::command().get_matches_from(itr))
    }
    pub async fn run(&mut self) -> Result<(), Error> {
        match &mut self.cmd {
            Cmd::Init(init_info) => init_info.run(&self.global_args).await?,
            Cmd::Setup(setup_info) => {
                setup_info.run(&self.global_args).await?;
            }
            Cmd::Version(version_info) => version_info.run(),
            Cmd::Build(build_info) => build_info.run(&self.global_args).await?,
            Cmd::Generate(generate) => match &mut generate.cmd {
                generate::Command::Contract(contract) => contract.run(&self.global_args).await?,
            },
            Cmd::Ext(ext_cmd) => match &ext_cmd.cmd {
                ext::Command::Ls(ls) => ls.run(&self.global_args).map_err(ext::Error::from)?,
            },
            Cmd::Upgrade(upgrade_info) => upgrade_info.run(&self.global_args).await?,
            Cmd::UpdateEnv(e) => e.run()?,
            Cmd::Watch(watch_info) => watch_info.run(&self.global_args).await?,
            Cmd::Clean(clean) => clean.run(&self.global_args)?,
        }
        Ok(())
    }
}

impl FromStr for Root {
    type Err = clap::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_arg_matches(s.split_whitespace())
    }
}

#[derive(Parser, Debug)]
pub enum Cmd {
    /// Initialize the project
    Init(init::Cmd),
    /// Set up an existing project (copy .env, install extensions, build contracts, git init)
    Setup(setup::Cmd),
    /// Version of the scaffold-stellar-cli
    Version(version::Cmd),

    /// Build contracts, resolving dependencies in the correct order. If you have an `environments.toml` file, it will also follow its instructions to configure the environment set by the `STELLAR_SCAFFOLD_ENV` environment variable, turning your contracts into frontend packages (JS dependencies).
    Build(build::Command),

    /// generate contracts
    Generate(generate::Cmd),

    /// Inspect and manage extensions
    Ext(ext::Cmd),

    /// Upgrade an existing Soroban workspace to a scaffold project
    Upgrade(upgrade::Cmd),

    /// Update an environment variable in a .env file
    UpdateEnv(update_env::Cmd),

    /// Monitor contracts and environments.toml for changes and rebuild as needed
    Watch(watch::Cmd),

    /// Clean Scaffold-generated artifacts from the given workspace
    Clean(clean::Cmd),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // TODO: stop using Debug for displaying errors
    #[error(transparent)]
    Init(#[from] init::Error),
    #[error(transparent)]
    Setup(#[from] setup::Error),
    #[error(transparent)]
    BuildContracts(#[from] build::Error),
    #[error(transparent)]
    Contract(#[from] generate::contract::Error),
    #[error(transparent)]
    Ext(#[from] ext::Error),
    #[error(transparent)]
    Upgrade(#[from] upgrade::Error),
    #[error(transparent)]
    UpdateEnv(#[from] update_env::Error),
    #[error(transparent)]
    Watch(#[from] watch::Error),
    #[error(transparent)]
    Clean(#[from] clean::Error),
}

#[derive(serde::Deserialize)]
struct PackageJson {
    #[serde(rename = "packageManager")]
    package_manager: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PackageManagerSpec {
    pub kind: PackageManager,
    pub version: Option<String>,
}

impl PackageManagerSpec {
    pub fn write_to_package_json(&self, workspace_root: &Path) -> io::Result<()> {
        let pkg_path = workspace_root.join("package.json");
        let contents =
            read_to_string(&pkg_path).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let pm_field_value = match &self.version {
            Some(version) => format!("{}@{}", self.kind.as_str(), version),
            None => self.kind.as_str().to_string(),
        };

        let updated = set_package_manager_field(&contents, &pm_field_value)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "malformed package.json"))?;

        std::fs::write(&pkg_path, updated)?;
        Ok(())
    }

    pub fn from_package_json(workspace_root: &Path) -> Option<Self> {
        let pkg_path = workspace_root.join("package.json");
        let contents = read_to_string(pkg_path).ok()?;

        let pkg: PackageJson = serde_json::from_str(&contents).ok()?;
        let raw = pkg.package_manager?;

        Some(PackageManagerSpec::parse_package_manager_field(&raw))
    }

    // "pnpm@9.6.0" → PackageManagerSpec { kind: Pnpm, version: Some("9.6.0") }
    fn parse_package_manager_field(value: &str) -> Self {
        let mut parts = value.split('@');
        let name = parts.next().unwrap_or(value);
        let version = parts.next().map(std::string::ToString::to_string);

        let kind = match name {
            "pnpm" => PackageManager::Pnpm,
            "yarn" => PackageManager::Yarn,
            "bun" => PackageManager::Bun,
            "deno" => PackageManager::Deno,
            _ => PackageManager::Npm,
        };

        Self { kind, version }
    }
}

/// Replace or insert the `packageManager` field in a package.json string,
/// preserving the original formatting of the rest of the file.
fn set_package_manager_field(json: &str, value: &str) -> Option<String> {
    let re = Regex::new(r#""packageManager"\s*:\s*"[^"]*""#).ok()?;
    let replacement = format!(r#""packageManager": "{value}""#);

    if re.is_match(json) {
        Some(re.replace(json, replacement.as_str()).into_owned())
    } else {
        // Field absent — insert before the final closing brace
        let insert_pos = json.rfind('}')?;
        let before = &json[..insert_pos];
        let after = &json[insert_pos..];
        let before_trimmed = before.trim_end();
        let comma = if before_trimmed.ends_with(',') {
            ""
        } else {
            ","
        };
        Some(format!("{before_trimmed}{comma}\n  {replacement}\n{after}"))
    }
}

#[derive(Debug, Clone, PartialEq, clap::ValueEnum)]
pub enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Deno,
}

impl PackageManager {
    pub const LIST: &'static [Self] = &[Self::Npm, Self::Pnpm, Self::Yarn, Self::Bun, Self::Deno];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
            Self::Yarn => "yarn",
            Self::Bun => "bun",
            Self::Deno => "deno",
        }
    }

    pub fn command(&self) -> &'static str {
        match self {
            Self::Npm => Self::os_specific_command("npm"),
            Self::Pnpm => Self::os_specific_command("pnpm"),
            Self::Yarn => Self::os_specific_command("yarn"),
            // bun and deno ship as native binaries on all platforms, no .cmd wrapper needed
            Self::Bun | Self::Deno => self.as_str(),
        }
    }

    fn os_specific_command(base: &'static str) -> &'static str {
        if cfg!(target_os = "windows") {
            match base {
                "npm" => "npm.cmd",
                "pnpm" => "pnpm.cmd",
                "yarn" => "yarn.cmd",
                _ => base,
            }
        } else {
            base
        }
    }

    pub(crate) fn install_silent(&self, dir: &Path) -> io::Result<Output> {
        let mut cmd = Command::new(self.command());
        cmd.current_dir(dir);
        match self {
            Self::Npm => cmd.args(["install", "--loglevel=error"]),
            Self::Pnpm => cmd.args(["install", "--reporter=silent"]),
            Self::Yarn | Self::Bun => cmd.args(["install", "--silent"]),
            Self::Deno => cmd.args(["install", "--quiet"]),
        };
        cmd.output()
    }

    pub(crate) fn install_no_workspace(&self, dir: &Path) -> io::Result<Output> {
        let mut cmd = Command::new(self.command());
        cmd.current_dir(dir);
        match self {
            Self::Npm => cmd.args(["install", "--no-workspaces", "--loglevel=error"]),
            // pnpm: --ignore-workspace prevents picking up workspace config from parent dirs
            Self::Pnpm => cmd.args(["install", "--ignore-workspace", "--reporter=silent"]),
            // yarn classic: --ignore-workspace-root-check skips workspace root enforcement
            Self::Yarn => cmd.args(["install", "--ignore-workspace-root-check", "--silent"]),
            // bun and deno workspaces are opt-in via their config files, so temp dirs are
            // already isolated without extra flags
            Self::Bun => cmd.args(["install", "--silent"]),
            Self::Deno => cmd.args(["install", "--quiet"]),
        };
        cmd.output()
    }

    pub(crate) fn build(&self, dir: &Path) -> io::Result<Output> {
        let mut cmd = Command::new(self.command());
        cmd.current_dir(dir);
        match self {
            Self::Npm => cmd.args(["run", "build", "--loglevel=error"]),
            Self::Pnpm => cmd.args(["run", "build", "--reporter=silent"]),
            Self::Yarn | Self::Bun => cmd.args(["run", "build"]),
            // deno uses `task` not `run` to execute package.json scripts
            Self::Deno => cmd.args(["task", "build"]),
        };
        cmd.output()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_npm_with_version() {
        let spec = PackageManagerSpec::parse_package_manager_field("npm@11.0.0");
        assert!(matches!(spec.kind, PackageManager::Npm));
        assert_eq!(spec.version, Some("11.0.0".to_string()));
    }

    #[test]
    fn parse_pnpm_with_version() {
        let spec = PackageManagerSpec::parse_package_manager_field("pnpm@9.6.0");
        assert!(matches!(spec.kind, PackageManager::Pnpm));
        assert_eq!(spec.version, Some("9.6.0".to_string()));
    }

    #[test]
    fn parse_yarn_no_version() {
        let spec = PackageManagerSpec::parse_package_manager_field("yarn");
        assert!(matches!(spec.kind, PackageManager::Yarn));
        assert_eq!(spec.version, None);
    }

    #[test]
    fn parse_bun() {
        let spec = PackageManagerSpec::parse_package_manager_field("bun@1.1.0");
        assert!(matches!(spec.kind, PackageManager::Bun));
        assert_eq!(spec.version, Some("1.1.0".to_string()));
    }

    #[test]
    fn parse_unknown_defaults_to_npm() {
        let spec = PackageManagerSpec::parse_package_manager_field("cargo@1.0.0");
        assert!(matches!(spec.kind, PackageManager::Npm));
    }

    #[test]
    fn set_package_manager_field_replaces_existing() {
        let json = r#"{"name": "foo", "packageManager": "npm@10.0.0", "version": "1.0.0"}"#;
        let result = set_package_manager_field(json, "pnpm@9.6.0").unwrap();
        assert!(result.contains(r#""packageManager": "pnpm@9.6.0""#));
        assert!(result.contains(r#""name": "foo""#));
        assert!(result.contains(r#""version": "1.0.0""#));
    }

    #[test]
    fn set_package_manager_field_inserts_when_absent() {
        let json = "{\n  \"name\": \"foo\"\n}";
        let result = set_package_manager_field(json, "npm@11.0.0").unwrap();
        assert!(result.contains(r#""packageManager": "npm@11.0.0""#));
        assert!(result.contains(r#""name": "foo""#));
        assert!(serde_json::from_str::<serde_json::Value>(&result).is_ok());
    }

    #[test]
    fn set_package_manager_field_inserts_produces_valid_json() {
        // real-world multi-line package.json with trailing newline before closing brace
        let json = "{\n  \"name\": \"my-app\",\n  \"version\": \"1.0.0\"\n}";
        let result = set_package_manager_field(json, "pnpm@9.6.0").unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&result).is_ok());
        assert!(result.contains("\"packageManager\": \"pnpm@9.6.0\""));
    }

    #[test]
    fn set_package_manager_field_preserves_surrounding_content() {
        let json = r#"{"scripts": {"start": "vite"}, "packageManager": "npm@10.0.0"}"#;
        let result = set_package_manager_field(json, "bun@1.0.0").unwrap();
        assert!(result.contains(r#""scripts""#));
        assert!(result.contains(r#""packageManager": "bun@1.0.0""#));
    }

    mod parameterized {
        use super::*;
        use rstest::rstest;

        #[rstest]
        #[case(PackageManager::Npm, "npm")]
        #[case(PackageManager::Pnpm, "pnpm")]
        #[case(PackageManager::Yarn, "yarn")]
        #[case(PackageManager::Bun, "bun")]
        #[case(PackageManager::Deno, "deno")]
        fn as_str_matches_name(#[case] pm: PackageManager, #[case] expected: &str) {
            assert_eq!(pm.as_str(), expected);
        }

        #[rstest]
        #[case(PackageManager::Npm)]
        #[case(PackageManager::Pnpm)]
        #[case(PackageManager::Yarn)]
        #[case(PackageManager::Bun)]
        #[case(PackageManager::Deno)]
        fn command_contains_name(#[case] pm: PackageManager) {
            assert!(pm.command().contains(pm.as_str()));
        }

        #[rstest]
        #[case(PackageManager::Npm, Some("11.0.0".to_string()))]
        #[case(PackageManager::Pnpm, Some("9.6.0".to_string()))]
        #[case(PackageManager::Yarn, None)]
        #[case(PackageManager::Bun, Some("1.1.0".to_string()))]
        fn package_json_round_trip(#[case] pm: PackageManager, #[case] version: Option<String>) {
            let dir = tempfile::tempdir().unwrap();
            let pkg_path = dir.path().join("package.json");
            std::fs::write(&pkg_path, "{\n  \"name\": \"test-app\"\n}").unwrap();

            let spec = PackageManagerSpec { kind: pm, version };
            spec.write_to_package_json(dir.path()).unwrap();

            let contents = std::fs::read_to_string(&pkg_path).unwrap();
            assert!(serde_json::from_str::<serde_json::Value>(&contents).is_ok());

            let recovered = PackageManagerSpec::from_package_json(dir.path()).unwrap();
            assert_eq!(recovered.kind, spec.kind);
            assert_eq!(recovered.version, spec.version);
        }
    }
}
