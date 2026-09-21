use std::process::Command;

use crate::commands::build::docker;
use crate::commands::build::env_toml::ENV_FILE;
use crate::commands::doctor::diagnosis::{Category, Check, Context, Diagnosis};

/// Reason a network check did not apply, or `None` when it should run. The
/// local network is only expected to be up when an environment opts into it.
fn skip_reason(ctx: &Context<'_>) -> Option<&'static str> {
    if ctx.workspace_root.is_none() {
        return Some("not in a Cargo workspace");
    }
    match ctx.environment() {
        None => Some("no environment configured"),
        Some(env) if !env.network.run_locally => Some("network is not run locally"),
        Some(_) => None,
    }
}

/// Checks that the Docker daemon is reachable, not merely installed.
pub struct DockerDaemon;

#[async_trait::async_trait]
impl Check for DockerDaemon {
    fn name(&self) -> &'static str {
        "docker-daemon"
    }

    fn category(&self) -> Category {
        Category::Network
    }

    async fn run(&self, ctx: &Context<'_>) -> Vec<Diagnosis> {
        if let Some(reason) = skip_reason(ctx) {
            return vec![Diagnosis::skipped(self.name(), Category::Network, reason)];
        }

        // `docker info` talks to the daemon; `docker --version` only reads the
        // client binary and succeeds while the daemon is down.
        let diagnosis = match Command::new("docker").arg("info").output() {
            Ok(output) if output.status.success() => {
                Diagnosis::ok(self.name(), Category::Network, "daemon is running")
            }
            Ok(_) => Diagnosis::error(self.name(), Category::Network, "daemon is not responding")
                .with_fix("start Docker, then: stellar scaffold build"),
            Err(_) => Diagnosis::error(self.name(), Category::Network, "docker not found")
                .with_fix("install Docker from https://docs.docker.com/get-docker/"),
        };

        vec![diagnosis]
    }
}

/// Probes the local Stellar network's RPC and friendbot endpoints.
pub struct Localnet;

#[async_trait::async_trait]
impl Check for Localnet {
    fn name(&self) -> &'static str {
        "localnet"
    }

    fn category(&self) -> Category {
        Category::Network
    }

    async fn run(&self, ctx: &Context<'_>) -> Vec<Diagnosis> {
        if let Some(reason) = skip_reason(ctx) {
            return vec![Diagnosis::skipped(self.name(), Category::Network, reason)];
        }

        let report = docker::probe_local_health().await;

        let rpc = match report.rpc_status.as_deref() {
            Some("healthy") => Diagnosis::ok("localnet-rpc", Category::Network, "RPC is healthy"),
            Some(status) => Diagnosis::warn(
                "localnet-rpc",
                Category::Network,
                format!("RPC reports status {status}"),
            )
            .with_fix("wait for the network to sync, or restart it"),
            None => Diagnosis::error("localnet-rpc", Category::Network, "RPC is not responding")
                .with_fix(start_fix()),
        };

        let friendbot = if report.friendbot {
            Diagnosis::ok("localnet-friendbot", Category::Network, "friendbot is up")
        } else {
            Diagnosis::error(
                "localnet-friendbot",
                Category::Network,
                "friendbot is not responding",
            )
            .with_fix(start_fix())
        };

        vec![rpc, friendbot]
    }
}

fn start_fix() -> String {
    format!("start the local network: stellar container start local (configured by {ENV_FILE})")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::build::clients::ScaffoldEnv;
    use crate::commands::build::env_toml;
    use crate::commands::doctor::diagnosis::{NO_COMMANDS, Severity};
    use std::path::Path;
    use stellar_cli::print::Print;

    fn context<'a>(root: Option<&'a Path>, printer: &'a Print) -> Context<'a> {
        Context {
            workspace_root: root,
            commands: &*NO_COMMANDS,
            env: ScaffoldEnv::Development,
            environment: root.map_or_else(
                || Ok(None),
                |root| env_toml::Environment::get(root, &ScaffoldEnv::Development),
            ),
            package_names: Vec::new(),
            printer,
        }
    }

    fn with_env_toml(contents: &str) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join(env_toml::ENV_FILE), contents).unwrap();
        dir
    }

    #[tokio::test]
    async fn skipped_outside_workspace() {
        let printer = Print::new(true);
        let ctx = context(None, &printer);
        assert_eq!(DockerDaemon.run(&ctx).await[0].severity, Severity::Skipped);
        assert_eq!(Localnet.run(&ctx).await[0].severity, Severity::Skipped);
    }

    #[tokio::test]
    async fn skipped_when_no_environment() {
        let dir = tempfile::TempDir::new().unwrap();
        let printer = Print::new(true);
        let ctx = context(Some(dir.path()), &printer);
        assert_eq!(DockerDaemon.run(&ctx).await[0].severity, Severity::Skipped);
        assert_eq!(Localnet.run(&ctx).await[0].severity, Severity::Skipped);
    }

    #[tokio::test]
    async fn skipped_when_network_is_remote() {
        let dir = with_env_toml("development.network = { name = \"testnet\" }\n");
        let printer = Print::new(true);
        let ctx = context(Some(dir.path()), &printer);

        // A remote network needs no local daemon, so neither check should run.
        let docker = DockerDaemon.run(&ctx).await;
        assert_eq!(docker[0].severity, Severity::Skipped);
        assert_eq!(docker[0].message, "network is not run locally");
        assert_eq!(Localnet.run(&ctx).await[0].severity, Severity::Skipped);
    }

    #[test]
    fn run_locally_clears_the_skip() {
        let dir = with_env_toml("development.network = { name = \"local\", run-locally = true }\n");
        let printer = Print::new(true);
        assert_eq!(skip_reason(&context(Some(dir.path()), &printer)), None);
    }
}
