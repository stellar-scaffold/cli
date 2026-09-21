use stellar_scaffold_cli::commands::build::docker;
use stellar_scaffold_test::{AssertExt, TestEnv, rpc_url};

/// The probe address is deliberately invalid, so a live friendbot answers 400.
/// Treating that as "not ready" would stall `start_local_stellar` against a
/// perfectly healthy network.
#[tokio::test]
async fn probe_reports_a_running_network_as_healthy() {
    let endpoints = docker::LocalEndpoints::from_rpc_url(Some(&rpc_url()));
    let report = docker::probe_local_health(&endpoints).await;

    assert_eq!(
        report.rpc_status.as_deref(),
        Some("healthy"),
        "expected a healthy RPC, got {:?}",
        report.rpc_status
    );
    assert!(
        report.friendbot,
        "friendbot answers 400 for the invalid probe address; that still means it is up"
    );
    assert!(report.is_healthy());
}

#[tokio::test]
async fn doctor_reports_the_local_network_as_healthy() {
    TestEnv::from_async("soroban-init-boilerplate", async |env| {
        env.set_environments_toml(format!(
            r#"
[development]
network = {{ rpc-url = "{}", network-passphrase = "Standalone Network ; February 2017", run-locally = true }}
"#,
            rpc_url(),
        ));

        // Problems elsewhere in the fixture make doctor exit non-zero, so read
        // the report rather than asserting on the exit code.
        let stdout = env
            .scaffold("doctor")
            .arg("--json")
            .assert()
            .stdout_as_str();
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        for name in ["docker-daemon", "localnet-rpc", "localnet-friendbot"] {
            assert_eq!(
                severity(&report, name).as_deref(),
                Some("ok"),
                "expected {name} to be ok in: {stdout}"
            );
        }
    })
    .await;
}

#[tokio::test]
async fn doctor_skips_network_checks_for_a_remote_network() {
    TestEnv::from_async("soroban-init-boilerplate", async |env| {
        env.set_environments_toml(format!(
            r#"
[development]
network = {{ rpc-url = "{}", network-passphrase = "Standalone Network ; February 2017" }}
"#,
            rpc_url(),
        ));

        let stdout = env
            .scaffold("doctor")
            .arg("--json")
            .assert()
            .stdout_as_str();
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        // Without run-locally there is no local network to diagnose.
        for name in ["docker-daemon", "localnet"] {
            assert_eq!(
                severity(&report, name).as_deref(),
                Some("skipped"),
                "expected {name} to be skipped in: {stdout}"
            );
        }
    })
    .await;
}

fn severity(report: &serde_json::Value, name: &str) -> Option<String> {
    report["checks"]
        .as_array()?
        .iter()
        .find(|check| check["name"] == name)?["severity"]
        .as_str()
        .map(ToString::to_string)
}
