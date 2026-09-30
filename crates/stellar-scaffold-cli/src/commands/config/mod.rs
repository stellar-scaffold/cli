//! `stellar scaffold config`: validate and inspect `scaffold.yml`.

use clap::Parser;
use stellar_cli::commands::global;

pub mod check;
pub mod show;

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Parser, Debug, Clone)]
pub enum Command {
    /// Validate scaffold.yml (schema version 2) and exit non-zero on errors
    Check(check::Cmd),
    /// Print the fully resolved config for one network: `extends` applied,
    /// built-in defaults filled in, per-network contract overrides merged,
    /// and `${env.…}`/`${network.…}` substituted
    Show(show::Cmd),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Check(#[from] check::Error),
    #[error(transparent)]
    Show(#[from] show::Error),
}

impl Cmd {
    pub fn run(&self, global_args: &global::Args) -> Result<(), Error> {
        match &self.cmd {
            Command::Check(check) => check.run(global_args)?,
            Command::Show(show) => show.run()?,
        }
        Ok(())
    }
}
