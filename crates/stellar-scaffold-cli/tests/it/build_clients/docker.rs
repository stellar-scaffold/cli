use stellar_cli::{CommandParser, commands as cli};

/// Make sure that `start_local_stellar` is idempotent. It shouldn't fail if container
/// is already running, it should skip startup logic and proceed to health checks.
#[tokio::test]
async fn start_local_stellar_is_idempotent() {
    // Test harness already sets up network, so this is _second_ call
    let result = stellar_scaffold_cli::commands::build::docker::start_local_stellar().await;
    assert!(
        result.is_ok(),
        "start_local_stellar should succeed when the container is already running, got: {result:?}"
    );
}

/// Directly invoke the underlying `container start` CLI command and assert that
/// the error message still contains at least one of the substrings we rely on
/// to detect an already-running container.
///
/// See: <https://github.com/stellar-scaffold/cli/pull/392>
#[tokio::test]
async fn container_start_error_contains_expected_substring() {
    let cmd = cli::container::StartCmd::parse_arg_vec(&["local"])
        .expect("failed to parse container start command");

    let result = cmd
        .run(&stellar_cli::commands::global::Args::default())
        .await;

    // Verify that the error matches what we're expecting
    if let Err(e) = result {
        let msg = e.to_string();
        assert!(
            msg.contains("already running") || msg.contains("port is already allocated"),
            "Container start error message changed! \
             Expected it to contain \"already running\" or \"port is already allocated\", \
             but got: \"{msg}\""
        );
    }
}
