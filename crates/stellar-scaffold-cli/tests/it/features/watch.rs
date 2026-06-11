use std::process::Stdio;
use stellar_scaffold_cli::commands::PackageManager;
use stellar_scaffold_test::{TestEnv, rpc_url};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::{StreamExt, wrappers::LinesStream};

#[ignore = "Need to fix but was a blocker"]
#[tokio::test]
async fn watch_command_watches_for_changes_and_environments_toml() {
    TestEnv::from_async("soroban-init-boilerplate", |env| async {
        Box::pin(async move {
            let mut watch_process = env
                .stellar_scaffold_process("watch", &["--build-clients"])
                .current_dir(&env.cwd)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to spawn watch process");

            let stderr = watch_process.stderr.take().unwrap();
            let mut stderr_lines = LinesStream::new(BufReader::new(stderr).lines());

            // Wait for initial build to complete
            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Watching for changes. Press Ctrl+C to stop.",
            )
            .await;

            // Test 1: Modify a source file
            let file_changed = "contracts/hello_world/src/lib.rs";
            env.modify_file(file_changed, "// This is a test modification");
            let file_changed_path = env.cwd.join(file_changed);

            // Wait for the watch process to detect changes and rebuild
            TestEnv::wait_for_output(
                &mut stderr_lines,
                &format!("File changed: {file_changed_path:?}"),
            )
            .await;

            TestEnv::wait_for_output(&mut stderr_lines, "cargo rustc").await;

            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Watching for changes. Press Ctrl+C to stop.",
            )
            .await;

            // Test 2: Create and modify environments.toml
            env.set_environments_toml(
                r#"
development.accounts = [
    { name = "alice" },
]

[development.network]
rpc-url = "http://localhost:8000/rpc"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_hello_world_contract.client = true
soroban_increment_contract.client = true
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
            );

            // Wait for the watch process to detect changes and rebuild
            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Using network at http://localhost:8000/rpc",
            )
            .await;

            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Watching for changes. Press Ctrl+C to stop.",
            )
            .await;

            // Test 3: modify the network url in environments.toml
            env.set_environments_toml(
                r#"
development.accounts = [
    { name = "alice" },
]

[development.network]
rpc-url = "http://localhost:9000/rpc"
network-passphrase = "Standalone Network ; February 2017"

[development.contracts]
soroban_hello_world_contract.client = true
soroban_increment_contract.client = true
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
            );

            // Wait for the watch process to detect changes and rebuild
            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Using network at http://localhost:9000/rpc",
            )
            .await;

            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Watching for changes. Press Ctrl+C to stop.",
            )
            .await;

            // Test 4: remove environments.toml
            let file_changed = "environments.toml";
            env.delete_file(file_changed);

            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Watching for changes. Press Ctrl+C to stop.",
            )
            .await;

            watch_process
                .kill()
                .await
                .expect("Failed to kill watch process");
        })
        .await;
    })
    .await;
}

#[tokio::test]
async fn dev_command_start_local_stellar_with_run_locally() {
    TestEnv::from_async("soroban-init-boilerplate", |env| async {
        Box::pin(async move {
            // Set environments.toml with run_locally enabled
            env.set_environments_toml(format!(
                r#"
development.accounts = [
    {{ name = "alice" }},
]

[development.network]
rpc-url = "{}"
network-passphrase = "Standalone Network ; February 2017"
run-locally = true

[development.contracts]
soroban_hello_world_contract.client = true
soroban_increment_contract.client = true
soroban_custom_types_contract.client = false
soroban_auth_contract.client = false
soroban_token_contract.client = false
"#,
                rpc_url()
            ));

            let mut watch_process = env
                .stellar_scaffold_process("watch", &["--build-clients"])
                .current_dir(&env.cwd)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to spawn watch process");

            let stderr = watch_process.stderr.take().unwrap();
            let mut stderr_lines = LinesStream::new(BufReader::new(stderr).lines());

            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Starting local Stellar Docker container...",
            )
            .await;

            TestEnv::wait_for_output(
                &mut stderr_lines,
                "Local Stellar network is healthy and running.",
            )
            .await;

            watch_process
                .kill()
                .await
                .expect("Failed to kill watch process");
        })
        .await;
    })
    .await;
}

#[ignore = "Need to fix but was a blocker"]
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn watch_and_vite_integration_test() {
    TestEnv::from_init("test-project", |env| async {
        Box::pin(async move {
            env.copy_env();
            env.update_package_json_to_use_built_binary()
                .expect("Package json should be editable");
            // Install npm dependencies
            let npm_cmd = PackageManager::Npm.command();
            let npm_install_output = tokio::process::Command::new(npm_cmd)
                .args(["install"])
                .current_dir(&env.cwd)
                .output()
                .await
                .expect("Failed to run npm install");

            assert!(
                npm_install_output.status.success(),
                "npm install failed: {}",
                String::from_utf8_lossy(&npm_install_output.stderr)
            );

            // Start npm run dev (which runs watch and vite concurrently)
            let mut dev_process = tokio::process::Command::new(npm_cmd)
                .args(["run", "dev"])
                .current_dir(&env.cwd)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to spawn npm run dev");

            let stdout = dev_process.stdout.take().unwrap();
            let stderr = dev_process.stderr.take().unwrap();
            let mut stdout_lines = LinesStream::new(BufReader::new(stdout).lines());
            let mut stderr_lines = LinesStream::new(BufReader::new(stderr).lines());

            // Monitor both stdout and stderr for process output
            let (output_sender, mut output_receiver) = tokio::sync::mpsc::unbounded_channel();
            // Monitor stdout
            let stdout_monitor = {
                let sender = output_sender.clone();
                tokio::spawn(async move {
                    while let Some(line) = stdout_lines.next().await {
                        if let Ok(line) = line {
                            let _ = sender.send(("stdout".to_string(), line));
                        }
                    }
                })
            };

            // Monitor stderr
            let stderr_monitor = {
                let sender = output_sender.clone();
                tokio::spawn(async move {
                    while let Some(line) = stderr_lines.next().await {
                        if let Ok(line) = line {
                            let _ = sender.send(("stderr".to_string(), line));
                        }
                    }
                })
            };

            // Wait for both watch and vite to be ready
            let mut watch_ready = false;
            let mut vite_ready = false;
            let mut port_found = false;
            let mut vite_port = None;
            let mut vite_errors = Vec::new();

            let timeout_duration = tokio::time::Duration::from_secs(180);
            let start_time = tokio::time::Instant::now();

            while (!watch_ready || !vite_ready) && start_time.elapsed() < timeout_duration {
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(5),
                    output_receiver.recv(),
                )
                .await
                {
                    Ok(Some((source, line))) => {
                        println!("📝 [{source}] {line}");

                        // Check for watch ready
                        if !watch_ready
                            && line.contains("Watching for changes. Press Ctrl+C to stop.")
                        {
                            watch_ready = true;
                        }

                        // Check for vite ready
                        if !vite_ready && line.contains("ready in") {
                            vite_ready = true;
                        }

                        // Check for port used
                        if !port_found && line.contains("Local:") {
                            // Extract port from lines like "Local:   http://localhost:5173/"
                            if let Some(port) = extract_port_from_line(&line) {
                                vite_port = Some(port);
                            }
                            port_found = true;
                        }

                        // Check for vite errors
                        if line.contains("Internal server error")
                            || line.contains("Failed to resolve import")
                            || line.contains("Does the file exist?")
                        {
                            vite_errors.push(line.clone());
                        }
                    }
                    Ok(None) => {
                        panic!("Output channel closed unexpectedly");
                    }
                    Err(_) => {}
                }
            }

            assert!(
                watch_ready,
                "Watch process did not become ready within timeout"
            );
            assert!(
                vite_ready,
                "Vite server did not become ready within timeout"
            );

            let port = vite_port.unwrap_or(5173);

            // Wait a bit for any initial vite errors to surface
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

            // Collect any errors that occurred during startup
            while let Ok((_, line)) = output_receiver.try_recv() {
                if line.contains("Internal server error")
                    || line.contains("Failed to resolve import")
                    || line.contains("Does the file exist?")
                {
                    vite_errors.push(line.clone());
                }
            }

            let client = reqwest::Client::new();

            // Request the generated flattened Clients module that would surface
            // import errors (all Clients are now exported from this one file).
            let js_module_paths = ["/app-lib/clients/index.ts"];

            for module_path in js_module_paths {
                let _ = client
                    .get(format!("http://localhost:{port}{module_path}"))
                    .timeout(tokio::time::Duration::from_secs(10))
                    .send()
                    .await;

                // Give vite time to process and potentially log errors
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }

            // Give vite time to process the request and potentially generate errors
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            // Collect any additional errors that might have occurred
            while let Ok((_, line)) = output_receiver.try_recv() {
                if line.contains("Internal server error")
                    || line.contains("Failed to resolve import")
                    || line.contains("Does the file exist?")
                {
                    vite_errors.push(line.clone());
                }
            }
            assert!(
                vite_errors.is_empty(),
                "Vite errors detected during test execution. Errors found: {vite_errors:?}"
            );

            // Modify hello world source file and monitor for rebuild
            let hello_world_file = "contracts/hello_world/src/lib.rs";
            let modified_content = r#"#![no_std]
use soroban_sdk::{contract, contractimpl, vec, Env, String, Vec};

#[contract]
pub struct HelloContract;

#[contractimpl]
impl HelloContract {
    pub fn hello(env: Env, to: String) -> Vec<String> {
        vec![&env, String::from_str(&env, "Hello modified world"), to]
    }
}

mod test;
"#;

            env.modify_file(hello_world_file, modified_content);
            let file_changed_path = env.cwd.join(hello_world_file);

            // Monitor for file change detection and rebuild
            let mut file_change_detected = false;
            let mut rebuild_started = false;
            let mut rebuild_completed = false;
            let mut new_vite_errors = Vec::new();

            let client = reqwest::Client::new();
            let hello_world_client_path = "/app-lib/clients/index.ts";

            // Monitor for 60 seconds for the rebuild process
            let rebuild_timeout = tokio::time::Duration::from_secs(60);
            let rebuild_start_time = tokio::time::Instant::now();

            while rebuild_start_time.elapsed() < rebuild_timeout {
                // Check for output from watch process
                match tokio::time::timeout(
                    tokio::time::Duration::from_millis(500),
                    output_receiver.recv(),
                )
                .await
                {
                    Ok(Some((source, line))) => {
                        println!("📝 [{source}] {line}");

                        // Check for file change detection
                        if !file_change_detected
                            && line.contains("File changed:")
                            && line.contains(&format!("{file_changed_path:?}"))
                        {
                            file_change_detected = true;
                        }

                        // Check for rebuild start
                        if !rebuild_started && line.contains("cargo rustc") {
                            rebuild_started = true;
                        }

                        // Check for rebuild completion
                        if !rebuild_completed
                            && file_change_detected
                            && rebuild_started
                            && line.contains("Watching for changes. Press Ctrl+C to stop.")
                        {
                            rebuild_completed = true;
                        }

                        // Check for new vite errors during rebuild
                        if line.contains("Internal server error")
                            || line.contains("Failed to resolve import")
                            || line.contains("Does the file exist?")
                        {
                            new_vite_errors.push(line.clone());
                        }
                    }
                    Ok(None) => {
                        panic!("Output channel closed unexpectedly during rebuild test");
                    }
                    Err(_) => {
                        // Timeout occurred, periodically query the hello world client file
                        if file_change_detected {
                            match client
                                .get(format!("http://localhost:{port}{hello_world_client_path}"))
                                .timeout(tokio::time::Duration::from_secs(5))
                                .send()
                                .await
                            {
                                Ok(response) => {
                                    if response.status().is_success() {
                                        println!(
                                            "✅ Hello world client file accessible during rebuild"
                                        );
                                    } else {
                                        println!(
                                            "⚠️  Hello world client file returned status: {}",
                                            response.status()
                                        );
                                    }
                                }
                                Err(e) => {
                                    println!("⚠️  Error querying hello world client file: {e}");
                                }
                            }
                        }
                        continue;
                    }
                }

                // Break if rebuild is complete
                if rebuild_completed {
                    break;
                }
            }

            assert!(
                file_change_detected,
                "File change was not detected by watch process"
            );

            assert!(
                rebuild_started,
                "Rebuild process did not start after file change"
            );

            assert!(
                rebuild_completed,
                "Rebuild process did not complete within timeout"
            );

            // Final check for any vite errors that occurred during rebuild
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            while let Ok((_, line)) = output_receiver.try_recv() {
                if line.contains("Internal server error")
                    || line.contains("Failed to resolve import")
                    || line.contains("Does the file exist?")
                {
                    new_vite_errors.push(line.clone());
                }
            }

            assert!(
                new_vite_errors.is_empty(),
                "Vite errors detected during hello world rebuild. Errors found: {new_vite_errors:?}"
            );

            // Final verification that hello world client is accessible after rebuild
            let final_response = client
                .get(format!("http://localhost:{port}{hello_world_client_path}"))
                .timeout(tokio::time::Duration::from_secs(10))
                .send()
                .await
                .expect("Failed to query hello world client after rebuild");

            assert!(
                final_response.status().is_success(),
                "Hello world client file not accessible after rebuild: {}",
                final_response.status()
            );

            println!("✅ Hello world modification and rebuild test completed successfully");

            // Cleanup
            stdout_monitor.abort();
            stderr_monitor.abort();

            if let Err(e) = dev_process.kill().await {
                eprintln!("Note: Error killing dev process: {e}");
            }
        })
        .await;
    })
    .await;
}

fn extract_port_from_line(line: &str) -> Option<u16> {
    // Look for patterns like "Local:   http://localhost:5173/" or "http://localhost:5174"
    if let Some(start) = line.find("localhost:") {
        let port_start = start + "localhost:".len();
        let port_end = line[port_start..]
            .find('/')
            .unwrap_or(line.len() - port_start);
        let port_str = &line[port_start..port_start + port_end];
        port_str.parse().ok()
    } else {
        None
    }
}
