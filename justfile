set dotenv-load := true

export PATH := './target/debug:./target/bin:' + env_var('PATH')
export CONFIG_DIR := 'target/'
export CI_BUILD := env_var_or_default('CI_BUILD', '')

[private]
path:
    just --list

scaffold +args:
    @cargo run --bin stellar-scaffold --quiet -- {{ args }}

reporter +args:
    @cargo run --bin stellar-scaffold-reporter --quiet -- {{ args }}

stellar-scaffold +args:
    @cargo run $CI_BUILD --bin stellar-scaffold -- {{ args }}

s +args:
    @stellar {{ args }}

stellar +args:
    @stellar {{ args }}

build_contract p:
    stellar contract build --profile contracts --package {{ p }}

# Build the CLI and the fixture test contracts
build:
    cargo build $CI_BUILD --package stellar-scaffold-cli
    just build-cli-test-contracts

# Setup the project to use a pinned version of the CLI
setup:
    git config core.hooksPath .githooks
    -cargo binstall -y stellar-cli --version 28.1.0 --force --install-path ./target/bin

# Build stellar-scaffold-cli test contracts to speed up testing
build-cli-test-contracts:
    just stellar-scaffold build --manifest-path crates/stellar-scaffold-test/fixtures/soroban-init-boilerplate/Cargo.toml

test: build
    cargo t

test-integration: build-cli-test-contracts
    cargo t --features integration-tests

[private]
_test-integration package filter ci="false":
    cargo t  -E 'package({{ package }}) and {{ filter }}' \
    {{ if ci == "false" { '--features integration-tests' } else { '--binaries-metadata target/nextest/binaries-metadata.json --cargo-metadata target/nextest/cargo-metadata.json --target-dir-remap target --workspace-remap .' } }}

[private]
_test-scaffold filter ci="false":
    just _test-integration stellar-scaffold-cli '{{ filter }}' {{ ci }}

# Run scaffold-cli accounts & contracts integration tests
test-integration-scaffold-build-clients ci="false":
    just _test-scaffold 'test(build_clients)' {{ ci }}

# Run scaffold-cli init_script, network, watch & clean integration tests
test-integration-scaffold-features ci="false":
    just _test-scaffold 'test(features::)' {{ ci }}

# Run scaffold-cli example integration tests (cases 1-14)
test-integration-scaffold-examples-1 ci="false":
    just _test-scaffold 'test(examples::) and test(/case_01/)' {{ ci }}
    just _test-scaffold 'test(examples::) and (test(/case_0[2-9]/) or test(/case_1[0-4]/))' {{ ci }}

# Run scaffold-cli example integration tests (cases 15-27)
test-integration-scaffold-examples-2 ci="false":
    just _test-scaffold 'test(examples::) and test(/case_15/)' {{ ci }}
    just _test-scaffold 'test(examples::) and (test(/case_1[6-9]/) or test(/case_2/))' {{ ci }}

# Run reporter integration tests
test-integration-reporter ci="false":
    just _test-integration stellar-scaffold-reporter 'test(/./)' {{ ci }}

clippy *args:
    cargo clippy --all {{ args }} \
    -- -Dclippy::pedantic -Aclippy::must_use_candidate -Aclippy::missing_errors_doc -Aclippy::missing_panics_doc

clippy-test:
    just clippy --tests --all-features

# Assert the running local network + stellar CLI match what this repo pins
preflight rpc_url=env_var_or_default('STELLAR_RPC_URL', 'http://localhost:8000/rpc'):
    #!/usr/bin/env bash
    set -euo pipefail
    # Catches a wrong quickstart image tag (network simulates the wrong protocol) or a
    # stale stellar-cli before integration tests deploy against it and fail with an
    # opaque simulation error. Assumes a local network is already up.
    docker_rs=crates/stellar-scaffold-cli/src/commands/build/docker.rs
    pinned_protocol=$(sed -n 's/^const LOCAL_PROTOCOL_VERSION: &str = "\(.*\)";/\1/p' "$docker_rs")
    pinned_cli=$(sed -n 's/.*stellar-cli = .*version = "\([^"]*\)".*/\1/p' Cargo.toml | head -1)

    # 1) The running network must simulate the pinned protocol.
    net=$(curl -fsS -X POST "{{ rpc_url }}" -H 'Content-Type: application/json' \
      -d '{"jsonrpc":"2.0","id":1,"method":"getNetwork"}')
    running=$(printf '%s' "$net" | sed -n 's/.*"protocolVersion":[[:space:]]*\([0-9]*\).*/\1/p')
    if [ "$running" != "$pinned_protocol" ]; then
      echo "::error::local network runs protocol '$running' but this build pins '$pinned_protocol'" \
           "— the quickstart image tag (LOCAL_IMAGE_TAG) likely doesn't match the protocol." >&2
      exit 1
    fi

    # 2) The stellar CLI on PATH must match the pinned major (its protocol line).
    installed=$(stellar --version | sed -n 's/^stellar \([0-9][0-9.]*\).*/\1/p' | head -1)
    if [ "${installed%%.*}" != "${pinned_cli%%.*}" ]; then
      echo "::error::stellar '$installed' on PATH but this build pins stellar-cli '$pinned_cli'." >&2
      exit 1
    fi

    echo "preflight OK: network protocol $running, stellar-cli $installed (pin $pinned_cli)."
