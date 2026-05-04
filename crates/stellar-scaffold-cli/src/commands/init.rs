use clap::Parser;
use degit::degit;
use std::fs::metadata;
use std::fs::read_dir;
use std::path::PathBuf;
use std::{env, io};

use super::setup;
use crate::commands::PackageManager;
use stellar_cli::{commands::global, print::Print};

pub const FRONTEND_TEMPLATE: &str = "theahaco/scaffold-stellar-frontend";

/// A command to initialize a new project
#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// The path to the project must be provided
    pub project_path: PathBuf,

    /// Template to clone, as a GitHub shorthand `user/repo`, optionally with
    /// a `#branch` or `#tag` suffix (e.g. `user/repo#my-branch`).
    /// Use `--template user/repo#tutorial` instead of the old `--tutorial` flag.
    #[arg(long, default_value = FRONTEND_TEMPLATE)]
    pub template: String,

    /// Specify package manager, omitting will prompt interactively
    #[arg(short = 'p', long)]
    pub package_manager: Option<PackageManager>,

    /// Accept all defaults and skip interactive prompts
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to clone template: {0}")]
    DegitError(String),
    #[error("Project path contains invalid UTF-8 characters and cannot be converted to a string")]
    InvalidProjectPathEncoding,
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    #[error(transparent)]
    SetupError(Box<setup::Error>),
}

impl From<setup::Error> for Error {
    fn from(e: setup::Error) -> Self {
        Self::SetupError(Box::new(e))
    }
}

impl Cmd {
    pub async fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);

        let absolute_project_path = self.project_path.canonicalize().unwrap_or_else(|_| {
            if self.project_path.is_absolute() {
                self.project_path.clone()
            } else {
                env::current_dir()
                    .unwrap_or_default()
                    .join(&self.project_path)
            }
        });

        printer.infoln(format!(
            "Creating new Stellar project in {}",
            absolute_project_path.display()
        ));

        let project_str = absolute_project_path
            .to_str()
            .ok_or(Error::InvalidProjectPathEncoding)?
            .to_owned();

        let repo = self.template.clone();
        tokio::task::spawn_blocking(move || {
            degit(repo.as_str(), &project_str);
        })
        .await
        .expect("Blocking task panicked");

        if metadata(&absolute_project_path).is_err()
            || read_dir(&absolute_project_path)?.next().is_none()
        {
            return Err(Error::DegitError(format!(
                "Failed to clone template into {}: directory is empty or missing",
                absolute_project_path.display()
            )));
        }

        let chosen_pm = setup::Cmd {
            project_path: absolute_project_path.clone(),
            package_manager: self.package_manager.clone(),
            yes: self.yes,
        }
        .run(global_args)
        .await?;

        let pm_command = chosen_pm.command();

        printer.blankln("\n\n");
        printer.checkln(format!(
            "Project successfully created at {}!",
            absolute_project_path.display()
        ));
        printer.blankln(" You can now run the application with:\n");
        printer.blankln(format!("\tcd {}", self.project_path.display()));
        printer.blankln(format!("\t{pm_command} start"));
        printer.blankln("\n Happy hacking! 🚀");

        Ok(())
    }
}
