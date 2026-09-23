//! `stellar scaffold doctor` — diagnose environment and configuration problems.
//!
//! - `diagnosis.rs` :: Severity, Diagnosis, Category, Context, Check
//! - `report.rs`    :: human + JSON rendering
//! - `checks/`      :: the checks themselves, registered in `checks::registry()`

pub mod checks;
pub mod diagnosis;
pub mod report;

use std::path::PathBuf;

use clap::Parser;
use stellar_cli::{commands::global, print::Print};

use crate::commands::build::clients::ScaffoldEnv;
use crate::commands::build::env_toml;
use diagnosis::{Context, Severity, SystemCommands};

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Scaffold environment to diagnose
    #[arg(
        long,
        env = "STELLAR_SCAFFOLD_ENV",
        value_enum,
        default_value = "development"
    )]
    pub env: ScaffoldEnv,

    /// Path to Cargo.toml
    #[arg(long)]
    pub manifest_path: Option<PathBuf>,

    /// Emit findings as JSON
    #[arg(long)]
    pub json: bool,

    /// Treat warnings as failures
    #[arg(long)]
    pub strict: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("doctor found {errors} problem(s); see the report above")]
    ProblemsFound { errors: usize },
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl Cmd {
    pub async fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);

        // A missing workspace isn't fatal: toolchain checks still apply, and the
        // rest report Skipped.
        let metadata = self.cargo_metadata();
        let workspace_root = metadata
            .as_ref()
            .map(|m| m.workspace_root.as_std_path().to_path_buf());

        let environment = workspace_root.as_deref().map_or_else(
            || Ok(None),
            |root| env_toml::Environment::get(root, &self.env),
        );

        let package_names = metadata
            .as_ref()
            .map(|m| m.packages.iter().map(|p| p.name.clone()).collect())
            .unwrap_or_default();

        let ctx = Context {
            workspace_root: workspace_root.as_deref(),
            commands: &SystemCommands,
            env: self.env,
            environment,
            package_names,
            printer: &printer,
        };

        let mut findings = Vec::new();
        for check in checks::registry() {
            findings.extend(check.run(&ctx).await);
        }

        if self.json {
            println!("{}", report::json(&findings)?);
        } else {
            report::human(&printer, &findings);
        }

        let mut errors = report::count(&findings, Severity::Error);
        if self.strict {
            errors += report::count(&findings, Severity::Warn);
        }
        if errors > 0 {
            return Err(Error::ProblemsFound { errors });
        }

        Ok(())
    }

    fn cargo_metadata(&self) -> Option<cargo_metadata::Metadata> {
        let mut command = cargo_metadata::MetadataCommand::new();
        if let Some(manifest_path) = &self.manifest_path {
            command.manifest_path(manifest_path);
        }
        command.no_deps().exec().ok()
    }
}
