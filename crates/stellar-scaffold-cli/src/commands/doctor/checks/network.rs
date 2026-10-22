use std::process::Command;

use crate::commands::build::docker::{self, LocalNetwork};
use crate::commands::doctor::diagnosis::{Category, Check, Context, Diagnosis};
use crate::config::CONFIG_FILE;

/// Every network this project runs in a local container, or the reason the
/// network checks don't apply. A version 2 project is checked for all of its
/// local networks, not just the one it's currently building for.
fn local_networks(ctx: &Context<'_>) -> Result<Vec<LocalNetwork>, String> {
    if ctx.workspace_root.is_none() {
        return Err("not in a Cargo workspace".into());
    }
    if let Some(loaded) = &ctx.config {
        let Some(config) = &loaded.config else {
            return Err(format!("{CONFIG_FILE} could not be read"));
        };
        let networks = docker::v2_local_networks(config);
        if networks.is_empty() {
            return Err(format!("no network in {CONFIG_FILE} runs locally"));
        }
        return Ok(networks);
    }
    match ctx.environment() {
        None => Err("no environment configured".into()),
        Some(env) => docker::env_toml_local_network(env, ctx.env)
            .map(|network| vec![network])
            .ok_or_else(|| "network is not run locally".into()),
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
        if let Err(reason) = local_networks(ctx) {
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
        let networks = match local_networks(ctx) {
            Ok(networks) => networks,
            Err(reason) => {
                return vec![Diagnosis::skipped(self.name(), Category::Network, reason)];
            }
        };

        // Networks sharing an RPC URL, e.g. one that extends `local`, run in
        // the same container, so each URL is probed once.
        let mut endpoints: Vec<(Option<String>, Vec<&str>)> = Vec::new();
        for network in &networks {
            match endpoints
                .iter_mut()
                .find(|(url, _)| *url == network.rpc_url)
            {
                Some((_, names)) => names.push(&network.name),
                None => endpoints.push((network.rpc_url.clone(), vec![&network.name])),
            }
        }

        let several = endpoints.len() > 1;
        let mut diagnoses = Vec::new();
        for (rpc_url, names) in endpoints {
            // Name the networks only when there's more than one container.
            let label = |message: &str| {
                if several {
                    format!("{}: {message}", names.join(", "))
                } else {
                    message.to_string()
                }
            };
            let report = docker::probe_local_health(&docker::LocalEndpoints::from_rpc_url(
                rpc_url.as_deref(),
            ))
            .await;

            diagnoses.push(match report.rpc_status.as_deref() {
                Some("healthy") => {
                    Diagnosis::ok("localnet-rpc", Category::Network, label("RPC is healthy"))
                }
                Some(status) => Diagnosis::warn(
                    "localnet-rpc",
                    Category::Network,
                    label(&format!("RPC reports status {status}")),
                )
                .with_fix("wait for the network to sync, or restart it"),
                None => Diagnosis::error(
                    "localnet-rpc",
                    Category::Network,
                    label("RPC is not responding"),
                )
                .with_fix(start_fix()),
            });
            diagnoses.push(if report.friendbot {
                Diagnosis::ok(
                    "localnet-friendbot",
                    Category::Network,
                    label("friendbot is up"),
                )
            } else {
                Diagnosis::error(
                    "localnet-friendbot",
                    Category::Network,
                    label("friendbot is not responding"),
                )
                .with_fix(start_fix())
            });
        }
        diagnoses
    }
}

fn start_fix() -> String {
    "start the local network: stellar container start local".into()
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
            config: None,
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
        assert!(local_networks(&context(Some(dir.path()), &printer)).is_ok());
    }

    /// A context for a version 2 project with `yaml` as its scaffold.yml.
    fn v2_context<'a>(root: &'a Path, yaml: &str, printer: &'a Print) -> Context<'a> {
        std::fs::write(root.join(CONFIG_FILE), yaml).unwrap();
        Context {
            config: Some(crate::config::load(root, Some(&[]), None)),
            ..context(Some(root), printer)
        }
    }

    #[test]
    fn v2_checks_local_beside_testnet() {
        // Whichever network the project is building for, it still needs a
        // working local container for `local`.
        let dir = tempfile::TempDir::new().unwrap();
        let printer = Print::new(true);
        let yaml = "version: 2\nnetworks:\n  local: {}\n  testnet: {}\n";
        let networks = local_networks(&v2_context(dir.path(), yaml, &printer)).unwrap();
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].name, "local");
    }

    #[tokio::test]
    async fn v2_remote_only_skips_the_local_container() {
        let dir = tempfile::TempDir::new().unwrap();
        let printer = Print::new(true);
        let yaml = "version: 2\nnetworks:\n  testnet: {}\n";
        let ctx = v2_context(dir.path(), yaml, &printer);

        let docker = DockerDaemon.run(&ctx).await;
        assert_eq!(docker[0].severity, Severity::Skipped);
        assert_eq!(docker[0].message, "no network in scaffold.yml runs locally");
        assert_eq!(Localnet.run(&ctx).await[0].severity, Severity::Skipped);
    }

    #[test]
    fn v2_ignores_environments_toml() {
        let dir = with_env_toml("development.network = { name = \"local\", run-locally = true }\n");
        let printer = Print::new(true);
        let yaml = "version: 2\nnetworks:\n  testnet: {}\n";
        assert!(local_networks(&v2_context(dir.path(), yaml, &printer)).is_err());
    }
}
