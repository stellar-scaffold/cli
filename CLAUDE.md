# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This repo is the **stellar-scaffold** CLI — a Stellar CLI plugin (`stellar scaffold`) for scaffolding dApps and smart contracts, plus its supporting Rust libraries. It is one of several repos split out of the original `scaffold-stellar` monorepo.

Related repos:
- `stellar-scaffold/ui` — frontend template fetched by `stellar scaffold init`
- `stellar-registry/cli` — the `stellar registry` CLI (consumes `stellar-build` and `stellar-scaffold-macro` published from this repo)
- `stellar-registry/contracts` — on-chain registry contracts

## Common Commands

```bash
# Install the pinned stellar-cli (v26.0.0) into ./target/bin and set up git hooks
just setup

# Build / check / test
cargo build --workspace
cargo check --workspace --all-targets
cargo test --workspace

# Run the CLI during development (stellar scaffold ...)
cargo run -p stellar-scaffold-cli -- <args>

# Format and lint
cargo fmt --all -- --check
cargo clippy --all-targets
```

Note: the `justfile` still carries some recipes from the monorepo (e.g. `just build` references the registry wasm). Prefer the `cargo` commands above until the justfile is trimmed to this repo.

## Code Quality Checklist

After making changes to Rust code, always run in order:
1. `cargo build -p <crate>` — confirm it compiles
2. `cargo test -p <crate>` — confirm tests pass (`cargo t` is aliased to `cargo nextest run`)
3. `cargo clippy -p <crate>` — fix any lint errors before considering work done

Clippy and warning flags are configured in `.cargo/config.toml` `rustflags` and apply automatically to every `cargo` invocation — no extra flags needed. `just clippy` runs across the whole workspace with additional allow-list overrides and matches CI.


## Architecture

### Crate Structure

| Crate | Purpose |
|-------|---------|
| `stellar-scaffold-cli` | The `stellar scaffold` CLI: `init`, `upgrade`, `build`, `generate`, `watch` |
| `stellar-build` | Contract building logic and dependency resolution (published to crates.io; also used by `stellar-registry/cli`) |
| `stellar-scaffold-macro` | Procedural macros (published to crates.io; also used by `stellar-registry/cli`) |
| `stellar-scaffold-ext-types` | Shared extension-hook types (serde) |
| `stellar-scaffold-test` | Test harness, fixtures, and integration utilities (`publish = false`; consumed via git by `stellar-registry/cli` tests) |
| `stellar-scaffold-reporter` | Built-in build-pipeline extension that logs compile/deploy metrics |

### Other directories

- `npm/` — npm wrapper that installs the prebuilt CLI binary
- `docs/site/` — Docusaurus documentation site (published to scaffoldstellar.com)
- `crates/stellar-scaffold-test/fixtures/` — fixture contracts and a boilerplate project for CLI integration tests

### CLI Command Flow

`init` → `build` → `generate` → `watch`
- `init` — scaffolds a new project; fetches the frontend template from `stellar-scaffold/ui` via degit
- `upgrade` — converts an existing Soroban workspace into a full scaffold project
- `build` — builds contracts and generates TypeScript clients based on `environments.toml`
- `generate contract` — adds a new contract to an existing project
- `watch` — rebuilds on changes

## Testing

- Unit tests run without external dependencies.
- Integration tests require a local Stellar RPC (Docker `stellar/quickstart`) and the `integration-tests` feature flag.
- Fixtures live in `crates/stellar-scaffold-test/fixtures/`.

## Build Profile

The fixture contracts use a custom `[profile.contracts]` with aggressive size optimization (`opt-level = "z"`, `lto = true`, `strip = "symbols"`). Build with `stellar contract build --profile contracts`.

## Cross-repo dependencies

`stellar-build` and `stellar-scaffold-macro` are published to crates.io and consumed by `stellar-registry/cli`. When bumping their public APIs, publish a new version so the registry repo can pick it up.
