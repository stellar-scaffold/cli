use std::error::Error;
use std::path::Path;
use stellar_cli::{CommandParser, commands as cli};

use super::clients::ScaffoldEnv;
use super::{env_toml, v2};
use crate::config::{self, resolve::Resolver};

/// Protocol version the local network is started with. Without this, quickstart
/// uses its default and contracts built against a newer soroban-sdk fail with
/// transaction simulation on deploy. Pinned explicitly and verified against the
/// running image by a preflight check.
const LOCAL_PROTOCOL_VERSION: &str = "29";

/// Quickstart image tag the local network runs. The floating `testing` tag
/// moves to new RPC releases, which only simulate the current and previous
/// protocol, so it must be pinned to an image that still simulates
/// [`LOCAL_PROTOCOL_VERSION`]. Bump it alongside the protocol. CI reads this
/// line to start the same image.
pub const LOCAL_IMAGE_TAG: &str = "v678-b1525.1-testing";

pub async fn start_local_stellar(rpc_url: Option<&str>) -> Result<(), Box<dyn Error>> {
    let result = cli::container::StartCmd::parse_arg_vec(&[
        "local",
        "--protocol-version",
        LOCAL_PROTOCOL_VERSION,
        "--image-tag-override",
        LOCAL_IMAGE_TAG,
    ])?
    .run(&stellar_cli::commands::global::Args::default())
    .await;
    if let Err(e) = result {
        if e.to_string().contains("already running")
            || e.to_string().contains("port is already allocated")
        {
            eprintln!("Container is already running, proceeding to health check...");
        } else {
            return Err(Box::new(e));
        }
    } else {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
    wait_for_local_health(
        &LocalEndpoints::from_rpc_url(rpc_url),
        std::time::Duration::from_secs(60),
    )
    .await
}

/// Cap on a single health request, so one hung socket can't stall the probe.
const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
/// Where `stellar container start local` listens, used when an environment does
/// not configure its own RPC URL
const DEFAULT_RPC_URL: &str = "http://localhost:8000/rpc";
/// The address is invalid on purpose; friendbot answering at all is the signal.
const FRIENDBOT_PROBE_ADDR: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

pub struct LocalEndpoints {
    rpc: String,
    friendbot: String,
}

impl LocalEndpoints {
    /// Derives both endpoints from env's `rpc-url` so a localnet on non-default
    /// port is probed where it actually listens.
    pub fn from_rpc_url(rpc_url: Option<&str>) -> Self {
        let rpc = rpc_url.unwrap_or(DEFAULT_RPC_URL).to_string();
        let friendbot = reqwest::Url::parse(&rpc).map_or_else(
            |_| rpc.clone(),
            |mut url| {
                url.set_path("/friendbot");
                url.set_query(Some(&format!("addr={FRIENDBOT_PROBE_ADDR}")));
                url.to_string()
            },
        );
        Self { rpc, friendbot }
    }
}

/// What one round of probes saw. An unreachable service is recorded, not raised
/// as an error, so the caller decides whether it's fatal.
pub struct HealthReport {
    /// Status string RPC reported, or `None` if it didn't answer.
    pub rpc_status: Option<String>,
    /// Whether friendbot responded at all.
    pub friendbot: bool,
}

impl HealthReport {
    /// Both services up and RPC reporting healthy.
    pub fn is_healthy(&self) -> bool {
        self.rpc_status.as_deref() == Some("healthy") && self.friendbot
    }
}

/// Probe RPC and friendbot once. Never retries; callers that want to wait loop
/// over this themselves.
pub async fn probe_local_health(endpoints: &LocalEndpoints) -> HealthReport {
    let Ok(client) = reqwest::Client::builder().timeout(PROBE_TIMEOUT).build() else {
        return HealthReport {
            rpc_status: None,
            friendbot: false,
        };
    };

    // Both run even if RPC is down, so a dead RPC doesn't hide friendbot's state.
    HealthReport {
        rpc_status: probe_rpc(&client, &endpoints.rpc).await,
        friendbot: probe_friendbot(&client, &endpoints.friendbot).await,
    }
}

async fn probe_rpc(client: &reqwest::Client, url: &str) -> Option<String> {
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(r#"{"jsonrpc": "2.0", "id": 1, "method": "getHealth"}"#)
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let json: serde_json::Value = response.json().await.ok()?;
    json["result"]["status"].as_str().map(ToString::to_string)
}

async fn probe_friendbot(client: &reqwest::Client, url: &str) -> bool {
    match client.get(url).send().await {
        // 400 is expected for the dummy address, and proves friendbot is up.
        Ok(response) => response.status().is_success() || response.status() == 400,
        Err(_) => false,
    }
}

async fn wait_for_local_health(
    endpoints: &LocalEndpoints,
    timeout: std::time::Duration,
) -> Result<(), Box<dyn Error>> {
    let start_time = std::time::Instant::now();

    loop {
        let report = probe_local_health(endpoints).await;
        if report.is_healthy() {
            return Ok(());
        }

        // Checked after probing so the attempt at the deadline still counts.
        if start_time.elapsed() > timeout {
            return Err(format!(
                "Health check timed out (rpc: {:?}, friendbot: {})",
                report.rpc_status, report.friendbot
            )
            .into());
        }

        eprintln!("Local network not ready, retrying health check.");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

/// A network that `build` runs in a local container, so it needs Docker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalNetwork {
    /// Its name under `networks:`, or the environments.toml environment.
    pub name: String,
    /// Its RPC URL, or `None` for quickstart's default or when the URL is
    /// only known after interpolating environment variables.
    pub rpc_url: Option<String>,
}

/// Every network the project at `root` runs in a local container, so it
/// needs Docker whichever network is being built right now. For a version 2
/// project that's each network under `networks:` that starts a container;
/// for environments.toml it's the `development` environment, the only one
/// `build` starts a container for.
pub fn local_networks(root: &Path) -> Vec<LocalNetwork> {
    if v2::is_v2(root) {
        return config::load(root, None, None)
            .config
            .map(|config| v2_local_networks(&config))
            .unwrap_or_default();
    }
    env_toml::Environment::get(root, &ScaffoldEnv::Development)
        .ok()
        .flatten()
        .and_then(|env| env_toml_local_network(&env, ScaffoldEnv::Development))
        .into_iter()
        .collect()
}

/// Each network of a version 2 config that starts a container, in the
/// order they're declared.
pub fn v2_local_networks(config: &config::Config) -> Vec<LocalNetwork> {
    config
        .networks
        .keys()
        .filter_map(|name| v2_local_network(config, name))
        .collect()
}

/// The network `name` of a version 2 config, if `build` starts a container
/// for it.
pub fn v2_local_network(config: &config::Config, name: &str) -> Option<LocalNetwork> {
    let net = Resolver::new(config).network(name)?;
    net.start_container().then(|| LocalNetwork {
        name: net.name.clone(),
        rpc_url: net
            .rpc_url
            .as_ref()
            .filter(|url| url.is_literal())
            .map(|url| url.raw.clone()),
    })
}

/// The network of environments.toml's `env`, if it runs locally.
pub fn env_toml_local_network(
    env: &env_toml::Environment,
    name: ScaffoldEnv,
) -> Option<LocalNetwork> {
    env.network.run_locally.then(|| LocalNetwork {
        name: name.to_string(),
        rpc_url: env.network.rpc_url.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Write `scaffold_yml` into a fresh project and load it.
    fn v2_config(scaffold_yml: &str) -> config::Config {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join(config::CONFIG_FILE), scaffold_yml).unwrap();
        config::load(dir.path(), None, None).config.unwrap()
    }

    #[test]
    fn local_runs_in_a_container() {
        let config = v2_config("version: 2\nnetworks:\n  local: {}\n");
        let local = v2_local_network(&config, "local").unwrap();
        assert_eq!(local.name, "local");
        assert!(local.rpc_url.is_some());
    }

    #[test]
    fn testnet_beside_local_needs_no_container() {
        // Building for testnet must not touch a local container, even when
        // the project also defines `local`.
        let config = v2_config("version: 2\nnetworks:\n  local: {}\n  testnet: {}\n");
        assert_eq!(v2_local_network(&config, "testnet"), None);
    }

    #[test]
    fn custom_network_follows_its_passphrase() {
        let config = v2_config(
            "version: 2\nnetworks:\n  sandbox:\n    rpc-url: http://localhost:9001/rpc\n    network-passphrase: Standalone Network ; February 2017\n",
        );
        let sandbox = v2_local_network(&config, "sandbox").unwrap();
        assert_eq!(
            sandbox.rpc_url.as_deref(),
            Some("http://localhost:9001/rpc")
        );
    }

    #[test]
    fn start_container_overrides_the_passphrase() {
        let config = v2_config(
            "version: 2\nnetworks:\n  local:\n    start-container: false\n  shared:\n    rpc-url: http://localhost:8000/rpc\n    network-passphrase: My Network\n    start-container: true\n",
        );
        assert_eq!(v2_local_network(&config, "local"), None);
        assert!(v2_local_network(&config, "shared").is_some());
    }

    #[test]
    fn templated_rpc_url_uses_the_default_endpoints() {
        let config = v2_config(
            "version: 2\nnetworks:\n  local:\n    rpc-url: http://${env.LOCAL_HOST}:8000/rpc\n",
        );
        assert_eq!(v2_local_network(&config, "local").unwrap().rpc_url, None);
    }

    #[test]
    fn every_local_network_is_listed() {
        // A project working on testnet still needs Docker for `local`.
        let config = v2_config(
            "version: 2\nnetworks:\n  testnet: {}\n  local: {}\n  sandbox:\n    extends: local\n",
        );
        let names: Vec<_> = v2_local_networks(&config)
            .into_iter()
            .map(|n| n.name)
            .collect();
        assert_eq!(names, ["local", "sandbox"]);
    }

    #[test]
    fn remote_only_project_needs_no_container() {
        let config = v2_config(
            "version: 2\nnetworks:\n  testnet: {}\n  mainnet:\n    rpc-url: https://rpc.example.com\n",
        );
        assert!(v2_local_networks(&config).is_empty());
    }

    #[test]
    fn unknown_network_needs_no_container() {
        let config = v2_config("version: 2\nnetworks:\n  local: {}\n");
        assert_eq!(v2_local_network(&config, "staging"), None);
    }

    #[test]
    fn friendbot_follows_the_configured_port() {
        let endpoints = LocalEndpoints::from_rpc_url(Some("http://localhost:9001/rpc"));
        assert_eq!(endpoints.rpc, "http://localhost:9001/rpc");
        assert_eq!(
            endpoints.friendbot,
            format!("http://localhost:9001/friendbot?addr={FRIENDBOT_PROBE_ADDR}")
        );
    }

    #[test]
    fn falls_back_to_the_quickstart_default() {
        let endpoints = LocalEndpoints::from_rpc_url(None);
        assert_eq!(endpoints.rpc, DEFAULT_RPC_URL);
        assert!(endpoints.friendbot.starts_with(&str::replace(
            DEFAULT_RPC_URL,
            "rpc",
            "friendbot"
        )));
    }
}
