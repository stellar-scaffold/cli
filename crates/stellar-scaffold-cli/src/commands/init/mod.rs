use clap::Parser;
use degit::degit;
use dialoguer::Select;
use dialoguer::theme::ColorfulTheme;
use std::fs::metadata;
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use std::{env, io};

use super::setup;
use crate::commands::PackageManager;
use stellar_cli::{commands::global, print::Print};

pub mod instantiate;

/// The official UI monorepo (Template Monorepo) degit'd by default.
pub const DEFAULT_UI_REPO: &str = "stellar-scaffold/ui";

/// Env var that overrides the UI monorepo degit target (repo plus optional
/// `#ref`). Points `init`/`upgrade` at an unreleased UI branch — chiefly used by
/// CI when landing coordinated cross-repo changes.
pub const UI_REPO_ENV: &str = "STELLAR_SCAFFOLD_UI_REPO";

/// Resolve the UI monorepo degit target, honoring [`UI_REPO_ENV`] when set and
/// non-empty, otherwise falling back to [`DEFAULT_UI_REPO`].
pub fn ui_repo() -> String {
    env::var(UI_REPO_ENV)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_UI_REPO.to_string())
}

/// A command to initialize a new project
#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// The path to the project must be provided
    pub project_path: PathBuf,

    /// Template selector. A bare framework name (e.g. `react`) picks an official
    /// template from the UI monorepo; a `user/repo` shorthand (optionally with a
    /// `#branch`/`#tag` suffix) degits that community repo directly. Omit to
    /// choose a framework interactively.
    #[arg(long)]
    pub template: Option<String>,

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
    Instantiate(#[from] instantiate::Error),
    #[error(transparent)]
    SetupError(Box<setup::Error>),
    #[error("No package manager selected")]
    NoPackageManager,
    #[error("Interactive selection cancelled")]
    Cancelled,
}

impl From<setup::Error> for Error {
    fn from(e: setup::Error) -> Self {
        Self::SetupError(Box::new(e))
    }
}

/// Which kind of source `init` is assembling from.
enum Source {
    /// Official UI monorepo; the `Option` holds an explicit framework, or `None`
    /// to choose one interactively once the monorepo is acquired.
    Official(Option<String>),
    /// Community repo shorthand, degit'd as-is.
    Community(String),
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

        // Resolve the source from --template using the slash heuristic.
        let source = match &self.template {
            None => Source::Official(None),
            Some(s) => match instantiate::parse_template_arg(s) {
                instantiate::TemplateSource::Framework(name) => Source::Official(Some(name)),
                instantiate::TemplateSource::Community(repo) => Source::Community(repo),
            },
        };

        let repo = match &source {
            Source::Official(_) => ui_repo(),
            Source::Community(repo) => repo.clone(),
        };

        // acquire: degit the chosen repo into the project path.
        acquire(&repo, &absolute_project_path).await?;

        // Gather the package-manager choice up front (both paths need it).
        let pkg_manager =
            setup::resolve_pkg_manager(self.package_manager.as_ref(), &printer, self.yes)
                .ok_or(Error::NoPackageManager)?;

        // instantiate: official templates promote one framework + apply pkg-mgr fs.
        match &source {
            Source::Official(framework) => {
                let framework = match framework {
                    Some(name) => name.clone(),
                    None => select_framework(&absolute_project_path, self.yes)?,
                };
                instantiate::instantiate(&absolute_project_path, &framework)?;
                instantiate::apply_package_manager(&absolute_project_path, &pkg_manager)?;
            }
            Source::Community(_) => {
                // Community repos own their own layout; just record the manager.
                let _ = pkg_manager.write_to_package_json(&absolute_project_path);
            }
        }

        // prepare: install, build, git.
        setup::prepare(&absolute_project_path, &pkg_manager, global_args, self.yes).await?;

        let pm_command = pkg_manager.kind.command();
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

/// acquire step: degit `repo` into `project_path` and verify it landed.
async fn acquire(repo: &str, project_path: &Path) -> Result<(), Error> {
    let project_str = project_path
        .to_str()
        .ok_or(Error::InvalidProjectPathEncoding)?
        .to_owned();
    let repo = repo.to_owned();
    tokio::task::spawn_blocking(move || {
        degit(repo.as_str(), &project_str);
    })
    .await
    .expect("Blocking task panicked");

    if metadata(project_path).is_err() || read_dir(project_path)?.next().is_none() {
        return Err(Error::DegitError(format!(
            "Failed to clone template into {}: directory is empty or missing",
            project_path.display()
        )));
    }
    Ok(())
}

/// Prompt for a framework from those available in the acquired monorepo, or pick
/// the first when running non-interactively.
fn select_framework(root: &Path, yes: bool) -> Result<String, Error> {
    let frameworks = instantiate::enumerate_templates(root)?;
    if frameworks.is_empty() {
        return Err(Error::Instantiate(instantiate::Error::NoTemplatesDir));
    }
    if yes || frameworks.len() == 1 {
        return Ok(frameworks[0].clone());
    }
    let index = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick a framework")
        .items(&frameworks)
        .default(0)
        .interact()
        .ok()
        .ok_or(Error::Cancelled)?;
    Ok(frameworks[index].clone())
}
