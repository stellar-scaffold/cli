use std::error::Error;
use stellar_cli::{CommandParser, commands as cli};

/// Protocol version the local network is started with. Without this, quickstart
/// arms its baked-in default (currently protocol 25), so contracts built against
/// a newer soroban-sdk fail transaction simulation on deploy. Derived at build
/// time from the `stellar-cli` pin in the workspace `Cargo.toml` (see build.rs),
/// so it tracks the protocol automatically when that dependency is bumped.
const LOCAL_PROTOCOL_VERSION: &str = env!("LOCAL_PROTOCOL_VERSION");

pub async fn start_local_stellar(rpc_url: Option<&str>) -> Result<(), Box<dyn Error>> {
    let result = cli::container::StartCmd::parse_arg_vec(&[
        "local",
        "--protocol-version",
        LOCAL_PROTOCOL_VERSION,
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

#[cfg(test)]
mod tests {
    use super::*;

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
