use clap::Parser;
use std::fmt::Debug;

#[derive(Parser, Debug, Clone)]
#[group(skip)]
pub struct Cmd;

impl Cmd {
    #[allow(clippy::unused_self)]
    pub fn run(&self) {
        println!("scaffold {}", long());
    }
}

pub fn pkg() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn git() -> &'static str {
    env!("GIT_REVISION")
}

pub fn long() -> String {
    let xdr = stellar_xdr::VERSION;

    [
        format!("{} ({})", pkg(), git()),
        format!(
            "stellar {} ({})",
            stellar_cli::commands::version::pkg(),
            stellar_cli::commands::version::git()
        ),
        format!(
            "stellar-xdr {} ({})
xdr ({})",
            xdr.pkg, xdr.rev, xdr.xdr,
        ),
    ]
    .join("\n")
}
