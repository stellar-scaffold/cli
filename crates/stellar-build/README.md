# stellar-build

A Rust library for discovering Stellar smart contract dependencies, ordering
packages for compilation, and locating network-specific Wasm artifacts. It also
provides helpers for Stellar network passphrases and IDs.

This crate supplies shared build infrastructure for
[Scaffold Stellar](../../README.md) and
[`stellar-scaffold-macro`](../stellar-scaffold-macro/). It does **not** compile
contracts, create output directories, deploy contracts, or provide a CLI. To build
contracts from the command line, use `stellar scaffold build`.

## Installation

Add the library to your Rust project's `Cargo.toml`:

```toml path=null start=null
[dependencies]
stellar-build = "0.0.7"
```

Dependency discovery invokes Cargo, so Rust and Cargo must be available in the
environment. The explicit `cargo tree` invocation uses the `CARGO` environment
variable when set, otherwise `cargo` on `PATH`.

## Marking contract dependencies

Add this metadata to each contract package's `Cargo.toml`:

```toml path=null start=null
[package.metadata.stellar]
contract = true
```

The marker must be the boolean `true`; a missing key, `false`, or a string such as
`"true"` is not recognized. `PackageExt::is_dep()` checks this marker.

Declare dependencies between contracts as normal Cargo dependencies. For example,
if `market` needs the Wasm built by `token`, the `market` manifest can contain:

```toml path=null start=null
[dependencies]
token = { path = "../token" }
```

The `token` package must carry the contract marker for discovery to recognize it
as a contract dependency. The marker alone does not add a dependency edge or
configure a package to compile to Wasm.

## Discovering contracts and ordering builds

Start from a package manifest, collect its marked contract dependencies, add the
root package, and sort the resulting packages so dependencies come first:

```rust path=null start=null
use std::path::Path;
use stellar_build::{all, contract, get_workspace};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = Path::new("contracts/market/Cargo.toml").canonicalize()?;

    // all() includes the root package, whether or not it has the marker.
    let root = all(&manifest)?
        .into_iter()
        .find(|package| package.manifest_path.as_std_path() == manifest.as_path())
        .expect("all() includes the root package");

    // contract() excludes the root package.
    let mut packages = contract(&manifest)?;
    packages.push(root);

    for package in get_workspace(&packages)? {
        println!("Build {} from {}", package.name, package.manifest_path);
        // The caller is responsible for compiling each package.
    }

    Ok(())
}
```

For `market` depending on `token`, this places `token` before `market`. Ordering
between unrelated packages is unspecified.

### Discovery behavior

- `all(manifest_path)` uses `cargo metadata` and
  `cargo tree --prefix none --edges normal` to collect dependencies, then appends
  the root package. It includes unmarked packages and may contain duplicates.
- `contract(manifest_path)` filters that list to packages marked with
  `package.metadata.stellar.contract = true`, excluding the supplied manifest.
  Discovery includes transitive normal dependencies, not build or dev dependencies.
- `get_workspace(packages)` orders the supplied packages using their contract
  dependencies. It returns only packages supplied by the caller; it does not add
  discovered dependencies to the result.

Use an absolute, canonical package manifest path, rather than a virtual workspace
manifest. Discovery requires a root package, and `contract()` excludes that
package by comparing manifest paths. The `cargo tree` subprocess runs from the
manifest's parent directory with Cargo's default package and feature selection;
these APIs do not accept feature, target, or package-selection options.

Discovery errors use `stellar_build::Error` (also available as `deps::Error`).
Malformed `cargo tree` output can panic, and the subprocess's nonzero exit status
is not explicitly checked. `get_workspace()` does not report cycles: if a graph
cannot be fully sorted, it can return a partial result.

## Wasm artifact paths

Artifacts follow the convention `<cargo-target-dir>/stellar/<network>/<name>.wasm`.
The network directory comes from `STELLAR_NETWORK`, defaulting to `local`, and
hyphens in the contract name are replaced with underscores.

```rust path=null start=null
use std::path::Path;
use stellar_build::{get_target_dir, stellar_wasm_out_dir, stellar_wasm_out_file};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cargo_target = Path::new("target");

    // With STELLAR_NETWORK unset:
    assert_eq!(
        stellar_wasm_out_dir(cargo_target),
        Path::new("target/stellar/local"),
    );
    assert_eq!(
        stellar_wasm_out_file(cargo_target, "my-token"),
        Path::new("target/stellar/local/my_token.wasm"),
    );

    // Resolve Cargo's configured target directory for an actual project.
    let artifact_dir = get_target_dir(Path::new("Cargo.toml"))?;
    println!("{}", artifact_dir.display());

    Ok(())
}
```

`stellar_wasm_out_dir()` and `stellar_wasm_out_file()` take the **base Cargo target
directory**. In contrast, `get_target_dir(manifest_path)` queries Cargo and returns
the **complete network-specific artifact directory**, already including
`stellar/<network>`. Do not pass its result back into the other path helpers.
None of these functions creates directories or checks for existing Wasm files.

The path helpers use `STELLAR_NETWORK` verbatim, without validating or lowercasing
it. They do not read `STELLAR_NETWORK_PASSPHRASE`.

## Network utilities

`Network` supports `Local` (the default), `Testnet`, `Futurenet`, and `Mainnet`.
Parsing a network name is case-insensitive:

```rust path=null start=null
use stellar_build::Network;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let network: Network = "testnet".parse()?;
    assert_eq!(network.passphrase(), "Test SDF Network ; September 2015");

    let network_id: [u8; 32] = network.id();
    assert_eq!(network_id.len(), 32);

    // Passphrase matching is exact.
    let mainnet = Network::from_passphrase(
        "Public Global Stellar Network ; September 2015",
    )?;
    assert!(matches!(mainnet, Network::Mainnet));

    Ok(())
}
```

- `Network::from_env()` reads `STELLAR_NETWORK`. An unset or invalid value falls
  back to `Local`.
- `Network::passphrase_from_env()` first tries `STELLAR_NETWORK_PASSPHRASE`. If it
  is unset or unrecognized, it falls back to `Network::from_env()`.
- `Network::from_passphrase()` recognizes the standard passphrases for the four
  supported networks; custom network passphrases are not supported.
- `passphrase()` returns the standard passphrase, and `id()` returns its SHA-256
  digest as 32 raw bytes.

For strict validation, use `"name".parse::<Network>()` or
`Network::from_passphrase()` instead of the environment helpers. These return
`stellar_build::networks::Error` for unrecognized inputs.

## License

Licensed under [Apache-2.0](../../LICENSE).

