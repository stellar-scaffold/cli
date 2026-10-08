use std::{
    fs::read_to_string,
    io,
    path::Path,
    process::{Command, Output},
    str::FromStr,
};

use clap::{CommandFactory, FromArgMatches, Parser};
use stellar_cli;

pub mod build;
pub mod clean;
pub mod config;
pub mod doctor;
pub mod ext;
pub mod generate;
pub mod init;
pub mod setup;
pub mod update_env;
pub mod upgrade;
pub mod version;
pub mod watch;

const ABOUT: &str = "Build smart contracts with frontend support";

/// Appended to failures whose cause is usually an environment or config
/// problem, which `doctor` can name precisely.
pub const DOCTOR_HINT: &str = "Run `stellar scaffold doctor` to diagnose.";

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
            Cmd::Doctor(doctor) => doctor.run(&self.global_args).await?,
            Cmd::Config(config) => config.run(&self.global_args)?,
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

    /// Diagnose environment and configuration problems in a scaffold project
    Doctor(doctor::Cmd),

    /// Validate and inspect scaffold.yml
    Config(config::Cmd),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    // TODO: stop using Debug for displaying errors
    #[error(transparent)]
    Init(#[from] init::Error),
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
    #[error(transparent)]
    Doctor(#[from] doctor::Error),
    #[error(transparent)]
    Config(#[from] config::Error),
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
    /// The package manager a project declares: `project.package-manager` in
    /// `scaffold.yml`, else the legacy `packageManager` field in
    /// `package.json`. `None` when the project names neither.
    pub fn declared(workspace_root: &Path) -> Option<Self> {
        build::scaffold_yml::ScaffoldConfig::get(workspace_root)
            .package_manager
            .map(|kind| Self {
                kind,
                version: None,
            })
            .or_else(|| Self::from_package_json(workspace_root))
    }

    /// The package manager to run for a project: its declared one, else npm.
    pub fn for_project(workspace_root: &Path) -> Self {
        Self::declared(workspace_root).unwrap_or(Self {
            kind: PackageManager::Npm,
            version: None,
        })
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

#[derive(thiserror::Error, Debug)]
pub enum EngineConstraintError {
    #[error(
        "This project requires stellar-scaffold {required}, but the installed version is {installed}. \
         Update with: cargo install stellar-scaffold-cli"
    )]
    ConstraintNotSatisfied { required: String, installed: String },
    #[error("Invalid engines.stellar-scaffold constraint '{constraint}': {source}")]
    InvalidConstraint {
        constraint: String,
        source: semver::Error,
    },
}

/// Read `engines.stellar-scaffold` from `package.json` and verify the running
/// CLI version satisfies it. Returns `Ok(())` if the file or field is absent.
pub fn check_engine_constraint(workspace_root: &Path) -> Result<(), EngineConstraintError> {
    #[derive(serde::Deserialize)]
    struct Engines {
        #[serde(rename = "stellar-scaffold")]
        stellar_scaffold: Option<String>,
    }
    #[derive(serde::Deserialize)]
    struct Pkg {
        engines: Option<Engines>,
    }

    let Ok(contents) = read_to_string(workspace_root.join("package.json")) else {
        return Ok(());
    };
    let Ok(pkg) = serde_json::from_str::<Pkg>(&contents) else {
        return Ok(());
    };
    let Some(constraint_str) = pkg.engines.and_then(|e| e.stellar_scaffold) else {
        return Ok(());
    };

    let req = semver::VersionReq::parse(&constraint_str).map_err(|e| {
        EngineConstraintError::InvalidConstraint {
            constraint: constraint_str.clone(),
            source: e,
        }
    })?;

    let installed =
        semver::Version::parse(version::pkg()).expect("CARGO_PKG_VERSION is always valid semver");

    if !req.matches(&installed) {
        return Err(EngineConstraintError::ConstraintNotSatisfied {
            required: constraint_str,
            installed: installed.to_string(),
        });
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, clap::ValueEnum, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
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
    fn declared_prefers_scaffold_yml_over_package_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"packageManager": "yarn@1.22.19"}"#,
        )
        .unwrap();
        let legacy = PackageManagerSpec::declared(dir.path()).unwrap();
        assert_eq!(legacy.kind, PackageManager::Yarn);
        assert_eq!(legacy.version, Some("1.22.19".to_string()));

        std::fs::write(
            dir.path().join("scaffold.yml"),
            "version: 2\nproject:\n  package-manager: pnpm\n",
        )
        .unwrap();
        let declared = PackageManagerSpec::declared(dir.path()).unwrap();
        assert_eq!(declared.kind, PackageManager::Pnpm);
    }

    #[test]
    fn for_project_defaults_to_npm() {
        let dir = tempfile::tempdir().unwrap();
        assert!(PackageManagerSpec::declared(dir.path()).is_none());
        assert_eq!(
            PackageManagerSpec::for_project(dir.path()).kind,
            PackageManager::Npm
        );
    }

    mod engine_constraint {
        use super::*;

        fn write_package_json(dir: &std::path::Path, contents: &str) {
            std::fs::write(dir.join("package.json"), contents).unwrap();
        }

        #[test]
        fn ok_when_no_package_json() {
            let dir = tempfile::tempdir().unwrap();
            assert!(check_engine_constraint(dir.path()).is_ok());
        }

        #[test]
        fn ok_when_no_engines_field() {
            let dir = tempfile::tempdir().unwrap();
            write_package_json(dir.path(), r#"{"name": "my-app"}"#);
            assert!(check_engine_constraint(dir.path()).is_ok());
        }

        #[test]
        fn ok_when_no_stellar_scaffold_engine() {
            let dir = tempfile::tempdir().unwrap();
            write_package_json(dir.path(), r#"{"engines": {"node": ">=18"}}"#);
            assert!(check_engine_constraint(dir.path()).is_ok());
        }

        #[test]
        fn ok_when_constraint_satisfied() {
            let dir = tempfile::tempdir().unwrap();
            // Current CLI version (from CARGO_PKG_VERSION) is 0.0.24, so >=0.0.1 passes.
            write_package_json(
                dir.path(),
                r#"{"engines": {"stellar-scaffold": ">=0.0.1"}}"#,
            );
            assert!(check_engine_constraint(dir.path()).is_ok());
        }

        #[test]
        fn err_when_constraint_not_satisfied() {
            let dir = tempfile::tempdir().unwrap();
            // Require a version far ahead of the current CLI.
            write_package_json(
                dir.path(),
                r#"{"engines": {"stellar-scaffold": ">=999.0.0"}}"#,
            );
            assert!(matches!(
                check_engine_constraint(dir.path()),
                Err(EngineConstraintError::ConstraintNotSatisfied { .. })
            ));
        }

        #[test]
        fn err_on_invalid_constraint() {
            let dir = tempfile::tempdir().unwrap();
            write_package_json(
                dir.path(),
                r#"{"engines": {"stellar-scaffold": "not-a-version"}}"#,
            );
            assert!(matches!(
                check_engine_constraint(dir.path()),
                Err(EngineConstraintError::InvalidConstraint { .. })
            ));
        }
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
    }
}
