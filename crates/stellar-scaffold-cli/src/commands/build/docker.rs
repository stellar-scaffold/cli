use std::error::Error;
use stellar_cli::{CommandParser, commands as cli};

/// Protocol version the local network is started with. Without this, quickstart
/// arms its baked-in default (currently protocol 25), so contracts built against
/// a newer soroban-sdk fail transaction simulation on deploy. Derived at build
/// time from the `stellar-cli` pin in the workspace `Cargo.toml` (see build.rs),
/// so it tracks the protocol automatically when that dependency is bumped.
const LOCAL_PROTOCOL_VERSION: &str = env!("LOCAL_PROTOCOL_VERSION");

pub async fn start_local_stellar() -> Result<(), Box<dyn Error>> {
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
    wait_for_stellar_health().await
}

async fn wait_for_stellar_health() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(60);

    // First check Stellar RPC health
    loop {
        let elapsed_time = start_time.elapsed();
        if elapsed_time > timeout {
            eprintln!("Timeout reached: stopping health checks.");
            return Err("Health check timed out".into());
        }
        let res = client
            .post("http://localhost:8000/rpc")
            .header("Content-Type", "application/json")
            .body(r#"{"jsonrpc": "2.0", "id": 1, "method": "getHealth"}"#)
            .send()
            .await?;
        if res.status().is_success() {
            let health_status: serde_json::Value = res.json().await?;
            if health_status["result"]["status"] == "healthy" {
                eprintln!("Stellar RPC is healthy, now checking friendbot...");
                break;
            }
            eprintln!("Stellar status is not healthy: {health_status:?}");
        } else {
            eprintln!("Health check request failed with status: {}", res.status());
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        eprintln!("Retrying health check.");
    }

    // Now check friendbot readiness
    loop {
        let elapsed_time = start_time.elapsed();
        if elapsed_time > timeout {
            eprintln!("Timeout reached: friendbot check failed.");
            return Err("Friendbot readiness check timed out".into());
        }

        // Use a dummy address to test friendbot availability
        let res = client
            .get("http://localhost:8000/friendbot?addr=GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF")
            .send()
            .await;

        match res {
            Ok(response) => {
                if response.status().is_success() || response.status() == 400 {
                    // 400 is expected for invalid address, but means friendbot is responding
                    eprintln!("Friendbot is ready!");
                    break;
                }
                eprintln!("Friendbot not ready, status: {}", response.status());
            }
            Err(e) => {
                eprintln!("Friendbot connection failed: {e}");
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        eprintln!("Retrying friendbot check.");
    }

    Ok(())
}
