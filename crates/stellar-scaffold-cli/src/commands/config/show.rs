//! `stellar scaffold config show`: print the resolved config for one network.

use std::path::PathBuf;

use clap::Parser;

use crate::commands::build::v2;
use crate::config::{self, NETWORK_ENV, Workspace};

#[derive(Parser, Debug, Clone)]
pub struct Cmd {
    /// Network to resolve. Defaults to `STELLAR_NETWORK`, then `stellar
    /// network use`, then `local`, the same as `build`
    #[arg(long, env = NETWORK_ENV)]
    pub network: Option<String>,

    /// Path to Cargo.toml
    #[arg(long)]
    pub manifest_path: Option<PathBuf>,

    /// Print JSON instead of YAML
    #[arg(long)]
    pub json: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{file} has errors; run `stellar scaffold config check` for details", file = config::CONFIG_FILE)]
    Invalid,
    #[error(transparent)]
    Yaml(#[from] serde_saphyr::ser::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let workspace = Workspace::discover(self.manifest_path.as_deref());
        // Contracts without an entry for this network are omitted rather than
        // reported; `config check --network` enforces that rule.
        let mut loaded = workspace.load(None);
        let Some(config) = loaded.config.as_ref().filter(|_| !loaded.has_errors()) else {
            loaded.diagnostics.retain(config::Diagnostic::is_error);
            eprint!("{}", loaded.render());
            return Err(Error::Invalid);
        };
        let env = |name: &str| std::env::var(name).ok();
        let (network, _) = v2::select_network(self.network.as_deref());
        match config::resolved_view(config, &network, &env) {
            Ok(view) if self.json => println!("{}", serde_json::to_string_pretty(&view)?),
            Ok(view) => print!("{}", serde_saphyr::to_string(&view)?),
            Err(diags) => {
                loaded.diagnostics = diags;
                eprint!("{}", loaded.render());
                return Err(Error::Invalid);
            }
        }
        Ok(())
    }
}
