//! `stellar scaffold config check`: validate `scaffold.yml`.
//!
//! Deterministic from repository contents: no network calls, no keystore, no
//! environment variables beyond network selection. Exits non-zero on any
//! error, so it can gate CI.

use std::path::PathBuf;

use clap::Parser;
use stellar_cli::{commands::global, print::Print};

use crate::config::{self, NETWORK_ENV, Severity, Workspace};

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Also require every contract to have an entry for this network
    #[arg(long, env = NETWORK_ENV)]
    pub network: Option<String>,

    /// Path to Cargo.toml
    #[arg(long)]
    pub manifest_path: Option<PathBuf>,

    /// Emit diagnostics as JSON
    #[arg(long)]
    pub json: bool,

    /// Treat warnings as failures
    #[arg(long)]
    pub strict: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0} found in {file}", file = config::CONFIG_FILE)]
    ProblemsFound(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl Cmd {
    pub fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        let printer = Print::new(global_args.quiet);
        let workspace = Workspace::discover(self.manifest_path.as_deref());
        let loaded = workspace.load(self.network.as_deref());

        if self.json {
            println!("{}", serde_json::to_string_pretty(&loaded.diagnostics)?);
        } else {
            eprint!("{}", loaded.render());
        }

        let count = |sev| {
            loaded
                .diagnostics
                .iter()
                .filter(|d| d.severity == sev)
                .count()
        };
        let (errors, warnings) = (count(Severity::Error), count(Severity::Warning));
        let failing = errors + if self.strict { warnings } else { 0 };
        if failing > 0 {
            return Err(Error::ProblemsFound(summary(errors, warnings)));
        }
        if !self.json {
            let suffix = if warnings > 0 {
                format!(" ({})", summary(0, warnings))
            } else {
                String::new()
            };
            printer.checkln(format!("{} is valid{suffix}", config::CONFIG_FILE));
        }
        Ok(())
    }
}

fn summary(errors: usize, warnings: usize) -> String {
    let plural = |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
    match (errors, warnings) {
        (0, w) => plural(w, "warning"),
        (e, 0) => plural(e, "error"),
        (e, w) => format!("{} and {}", plural(e, "error"), plural(w, "warning")),
    }
}
