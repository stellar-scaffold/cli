use std::path::Path;
use stellar_cli::print::Print;

use crate::commands::build::clients::ScaffoldEnv;
use crate::commands::build::env_toml;

/// Outcome of a single diagnostic. `Skipped` means the check did not apply
/// here (e.g. network isn't running locally), not that it passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Ok,
    Warn,
    Error,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Toolchain,
    Project,
    Network,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Self::Toolchain => "Toolchain",
            Self::Project => "Project",
            Self::Network => "Network",
        }
    }
}

/// One result. Optional `fix` is a one line copy-paste remedy, omitted when
/// result is `Ok` or if there is no single command to resolve it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Diagnosis {
    pub name: &'static str,
    pub category: Category,
    pub severity: Severity,
    pub message: String,
    pub fix: Option<String>,
}

impl Diagnosis {
    fn new(
        name: &'static str,
        category: Category,
        severity: Severity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            name,
            category,
            severity,
            message: message.into(),
            fix: None,
        }
    }

    pub fn ok(name: &'static str, category: Category, message: impl Into<String>) -> Self {
        Self::new(name, category, Severity::Ok, message)
    }

    pub fn warn(name: &'static str, category: Category, message: impl Into<String>) -> Self {
        Self::new(name, category, Severity::Warn, message)
    }

    pub fn error(name: &'static str, category: Category, message: impl Into<String>) -> Self {
        Self::new(name, category, Severity::Error, message)
    }

    pub fn skipped(name: &'static str, category: Category, message: impl Into<String>) -> Self {
        Self::new(name, category, Severity::Skipped, message)
    }

    /// Attach a remedy. Chained onto a warn/error constructor.
    #[must_use]
    pub fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }
}

/// Outcome of running an external command. `Failed` and `NotFound` are kept
/// apart because a tool that is installed but erroring needs a different fix
/// than one that is missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutput {
    /// Exited zero; carries trimmed stdout.
    Success(String),
    /// Ran but exited non-zero.
    Failed,
    /// Could not be run at all.
    NotFound,
}

impl CommandOutput {
    /// Stdout if the command succeeded.
    pub fn stdout(&self) -> Option<&str> {
        match self {
            Self::Success(out) => Some(out),
            _ => None,
        }
    }
}

pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> CommandOutput;
}

/// Runs commands for real. Swapped for a fake in tests.
pub struct SystemCommands;

impl CommandRunner for SystemCommands {
    fn run(&self, program: &str, args: &[&str]) -> CommandOutput {
        match std::process::Command::new(program).args(args).output() {
            Ok(output) if output.status.success() => {
                CommandOutput::Success(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            Ok(_) => CommandOutput::Failed,
            Err(_) => CommandOutput::NotFound,
        }
    }
}

/// A [`CommandRunner`] returning canned output, keyed by program name.
#[cfg(test)]
#[derive(Default)]
pub struct FakeCommands(std::collections::HashMap<String, CommandOutput>);

#[cfg(test)]
impl FakeCommands {
    /// Every program not otherwise registered reports as missing.
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with(mut self, program: &str, output: CommandOutput) -> Self {
        self.0.insert(program.to_string(), output);
        self
    }

    /// Shorthand for a program that exits zero with `stdout`.
    #[must_use]
    pub fn succeeding(self, program: &str, stdout: &str) -> Self {
        self.with(program, CommandOutput::Success(stdout.to_string()))
    }
}

#[cfg(test)]
impl CommandRunner for FakeCommands {
    fn run(&self, program: &str, _args: &[&str]) -> CommandOutput {
        self.0
            .get(program)
            .cloned()
            .unwrap_or(CommandOutput::NotFound)
    }
}

/// Shared fake for tests whose checks never shell out.
#[cfg(test)]
pub static NO_COMMANDS: std::sync::LazyLock<FakeCommands> =
    std::sync::LazyLock::new(FakeCommands::new);

/// Everything the checks read, resolved once before running so we don't need to
/// re-read/parse mid checks. `workspace_root` is `None` when `doctor` is called
/// outside a Cargo workspace (i.e. only checking toolchain) so project and
/// network checks report `Skipped`.
pub struct Context<'a> {
    pub workspace_root: Option<&'a Path>,
    /// How checks shell out, injected so they are testable without the real
    /// binaries on PATH.
    pub commands: &'a dyn CommandRunner,
    pub env: ScaffoldEnv,
    // TODO: replace with `config` when we migrate to scaffold.yml?
    /// Parse outcome of `environments.toml`. `Ok(None)` means the file is
    /// absent; the error is kept so a check can report a malformed file.
    pub environment: Result<Option<env_toml::Environment>, env_toml::Error>,
    /// Cargo package names in the workspace, for validating contract keys.
    pub package_names: Vec<String>,
    pub printer: &'a Print,
}

impl Context<'_> {
    /// The parsed environment, or `None` if absent or malformed.
    pub fn environment(&self) -> Option<&env_toml::Environment> {
        self.environment.as_ref().ok().and_then(Option::as_ref)
    }
}

#[async_trait::async_trait]
pub trait Check: Send + Sync {
    /// Stable identifier used as JSON key and human-readable label
    fn name(&self) -> &'static str;
    fn category(&self) -> Category;
    /// A check yields several results
    async fn run(&self, ctx: &Context<'_>) -> Vec<Diagnosis>;
}
