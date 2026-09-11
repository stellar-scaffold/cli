# Registry Guide

The Stellar Registry is a system for publishing, deploying, and managing smart contracts—and their underlying Wasms—on the Stellar network. This guide explains how to use the Registry CLI and UI to manage your contracts.

<div class="videoWrapper">
  <iframe src="https://www.youtube-nocookie.com/embed/xAlWmJOdMSQ?si=n2yYDkKbyqTAhiNP" title="Stellar Registry Full Walk-Through" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>
</div>

## Overview

At its core, Stellar Registry is a smart contract.

### Core Concepts

The Registry smart contract keeps track of two kinds of information:

1. **Wasms**: Stellar smart contracts are compiled to [WebAssembly](https://webassembly.org/), aka Wasm. The `.wasm` file you get at the end of a [`stellar scaffold build` command](./cli#build-command) needs to be uploaded to the Stellar blockchain. Without Registry, this is done with Stellar CLI using `stellar contract upload`.

   This gets your Wasm file/blob/binary/module on-chain, _identified only by its content hash._ This looks like a string of 65 hexadecimal characters like `d1d4e69…`.

   Stellar Registry allows you, the contract author, to also give that Wasm module a _name_ and a _version._ Key insight: Stellar is _already a module-distribution system_, like NPM or Crates.io. Registry makes it usable.

2. **Contracts:** Once a Wasm is on-chain, many contracts can use it. You can deploy a Contract for a Wasm you didn't write; someone else could deploy multiple Contracts all using your Wasm. If you're familiar with Object Oriented Programming, think of the Contract as the _instance_ and the Wasm as the _class._ A Wasm defines behavior; a Contract holds data (including a reference to the Wasm).

   :::note Contracts don't have versions

   You always interact with a Contract's live, latest version.

   :::

So that's it, at its core: Stellar Registry is a smart contract, which gives other smart contracts names, and which gives Wasms both names and versions.

### Quick Links

Around the core smart contract, Stellar Registry wraps other tooling:

- [Stellar Registry UI](https://rgstry.xyz), `rgstry.xyz`, where you can browse and search all Wasms and Contracts currently registered. [Source](https://github.com/stellar-registry/ui)
- [Stellar Registry CLI](https://crates.io/crates/stellar-registry-cli), the main interface used to _write_ to Stellar Registry today. This is the tool you use to _publish Wasms_ and _deploy Contracts_ to Stellar Registry, or to register already-uploaded Wasms and already-deployed Contracts. [Source](https://github.com/stellar-registry/cli)
- [Stellar Registry Rust Macros](https://crates.io/crates/stellar-registry), to simplify your cross-contract calls when authoring contracts. [Source](https://github.com/stellar-registry/cli/tree/main/crates/stellar-registry-macro)
- [Stellar Registry GitHub Actions](https://github.com/stellar-registry/actions), for creating [SEP-55 attested builds](https://github.com/stellar-expert/soroban-build-workflow) of your project Wasm files, uploading them to the blockchain, incrementing your Registry series' version, and auto-publishing your Wasm to Registry. [Coming soon](https://github.com/stellar-registry/oz-combined-wasms/issues/1)
- **Stellar Registry Governance**: to get your Contracts & Wasms into the root registry (no `unverified/` prefix; more on this below) or to get your own subregistry (your own prefix, like `oz/` or `circle/`, to which you can publish Wasms & deploy Contracts freely), you need to put in a request with the Stellar Registry security council. The Stellar Registry UI contains forms for doing this ([coming soon](https://github.com/stellar-registry/ui/issues/51)), but if you want to do so in the underlying system itself:
  - on Testnet, [use Tansu](https://testnet.tansu.dev/governance/?name=stellarregistry)
  - on Mainnet, use [github.com/stellar-registry/gov](https://github.com/stellar-registry/gov/issues) (Tansu coming soon)
- Stellar Registry API [testnet](https://stellar-registry-testnet.fly.dev/) and [mainnet](https://stellar-registry-mainnet.fly.dev/), the backends for Stellar Registry UI. Use at your own risk: domains may change and rate-limiting schemes may harden. [Source](https://github.com/stellar-registry/indexer)

Next, let's learn more about that `unverified/` prefix.

### Subregistries

Did we say Registry was _one_ contract? This isn't quite true. The _root_ Registry is one contract, but some of the contracts that it tracks are themselves Registries, running the same [Registry Wasm](https://stellar.rgstry.xyz/wasms/registry). These are _subregistries_.

Most, like the root Registry, are locked down, _managed_ Registries. You cannot publish Wasms or deploy Contracts to the `oz` or `circle` subregistries, for obvious security reasons.

The `unverified` subregistry, though, is _unmanaged._ Anyone can publish and deploy to it. As its name indicates, you _should not trust Wasms and Contracts in this subregistry._ If you see the `unverified` prefix on a Wasm/Contract name, _be careful!_

Some popular subregistries, to help you get a sense of how this all works:

- [`oz/`](https://stellar.rgstry.xyz/wasms?query=oz), the Open Zeppelin Wasms (managed [by the Stellar Registry team](https://stellar.rgstry.xyz/wasms?query=oz))
- [`circle/`](https://stellar.rgstry.xyz/contracts?query=circle), containing the official [`usdc`](https://stellar.rgstry.xyz/contracts/circle/usdc) and [`eurc`](https://stellar.rgstry.xyz/contracts/circle/eurc) tokens
- `unverified/`: check out [Testnet Wasms](https://testnet.rgstry.xyz/wasms?query=unverified) and [Testnet Contracts](https://testnet.rgstry.xyz/contracts?query=unverified) as well as [Mainnet Wasms](https://stellar.rgstry.xyz/wasms?query=unverified) and [Mainnet Contracts](https://stellar.rgstry.xyz/contracts?query=unverified) to get a sense of how noisy this subregistry can be!

When you first publish or deploy to Stellar Registry, you will need to use the `unverified` subregistry.

This:

```bash
# ✅ WORKS
stellar registry publish --wasm-name unverified/my-wasm
```

Not this:

```bash
# ❌ DOES NOT WORK
stellar registry publish --wasm-name my-wasm
```

:::tip

For full working example of the `publish` command, [see below](#using-the-unverified-registry).

:::

Then, once you're ready, you can use the [governance process linked above](#quick-links) to get into the root Registry, into a different subregistry, or to get your own subregistry.

### Name Resolution

Names in the registry support namespace prefixes. The CLI resolves names using the root registry as the source of truth:

- `my-contract` - Looks up in the verified (root) registry
- `unverified/my-contract` - First fetches the `unverified` registry contract ID from the root registry, then looks up `my-contract` in that registry
- `some-other-subregistry/my-contract` — Same as above. Looks up `some-other-subregistry` in the root registry, then looks up `my-contract` in that registry.

### Name Normalization

All names are normalized by the [stellar-registry-name](https://crates.io/crates/stellar-registry-name) crate before storage:

- Underscores (`_`) are converted to hyphens (`-`)
- Uppercase letters are converted to lowercase
- Names must start with an alphabetic character
- Names can only contain alphanumeric characters, hyphens, or underscores
- Rust keywords are not allowed as names
- Names have a maximum length of 64 characters

## Prerequisites

Install the registry CLI:

```bash
cargo install --locked stellar-registry-cli
```

As this currently embeds Stellar CLI as a dependency, it takes some time. To make it faster, you can use [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```bash
cargo install cargo-binstall
cargo binstall stellar-registry-cli
```

## Commands

### Publish Contract

Stellar Registry prefers the verb _publish_, rather than the more generic "upload." You're not merely uploading a Wasm blob to the blockchain; you're publishing a module. Like other module-distribution systems, you then keep control of this name, and the rights to publish new versions in the series.

Publish a compiled contract to the Stellar Registry:

```bash
stellar registry publish \
  --wasm <PATH_TO_WASM> \
  [--author <AUTHOR_ADDRESS>] \
  [--wasm-name <NAME>] \
  [--binver <VERSION>] \
  [--dry-run]
```

Options:

- `--wasm`: Path to the compiled WASM file (required)
- `--author (-a)`: Author address (optional, defaults to the configured source account)
- `--wasm-name`: Name for the published contract, supports prefix notation like `unverified/my-contract` (optional, extracted from contract metadata if not provided)
- `--binver`: Binary version (optional, extracted from contract metadata if not provided)
- `--dry-run`: Simulate the publish operation without actually executing it (optional)

**Note:** For the root registry, the governance process ([see above](#quick-links)) must approve initial publishes. For the unverified registry, use the `unverified/` prefix.

### Deploy Contract

Deploy a published contract with optional initialization parameters:

```bash
stellar registry deploy \
  --contract-name <DEPLOYED_NAME> \
  --wasm-name <PUBLISHED_NAME> \
  [--version <VERSION>] \
  [--deployer <DEPLOYER_ADDRESS>] \
  -- \
  [CONSTRUCTOR_ARGS...]
```

Options:

- `--contract-name`: The name to give this contract instance, supports prefix notation like `unverified/my-instance` (required)
- `--wasm-name`: The name of the previously published contract to deploy, supports prefix notation (required)
- `--version`: Specific version of the published contract to deploy (optional, defaults to most recent version)
- `--deployer`: Optional deployer address for deterministic contract ID resolution (advanced feature)
- `-- [CONSTRUCTOR_ARGS]`: `--` is a standard CLI separator between "arguments for the main process" and "arguments for the sub-process". `stellar registry deploy` creates an "implicit CLI" for the deployed contract, passing `CONSTRUCTOR_ARGS` to it. If the Wasm being deployed does not have a `__constructor`, you do not need this.

**Note:** For the root registry, the governance process ([see above](#quick-links)) must approve initial publishes. For the unverified registry, use the `unverified/` prefix.

### Deploy Unnamed Contract

Deploy a published contract without registering a name in the registry. This is useful when you want to deploy a contract but don't need name resolution. You can do this with the "Deploy a contract using this Wasm" button on any Wasm page in https://stellar.rgstry.xyz/wasms, or to use the CLI:

```bash
stellar registry deploy-unnamed \
  --wasm-name <PUBLISHED_NAME> \
  [--version <VERSION>] \
  [--salt <HEX_SALT>] \
  [--deployer <DEPLOYER_ADDRESS>] \
  -- \
  [CONSTRUCTOR_ARGS...]
```

Options:

- `--wasm-name`: The name of the previously published contract to deploy, supports prefix notation like `unverified/my-contract` (required)
- `--version`: Specific version of the published contract to deploy (optional, defaults to most recent version)
- `--salt`: Optional hex-encoded 32-byte salt for deterministic contract ID. If not provided, a random salt is used
- `--deployer`: Deployer account for deterministic contract ID resolution (optional)
- `CONSTRUCTOR_ARGS`: Optional arguments for the constructor function

Note: Use `--` to separate CLI options from constructor arguments.

### Register Existing Contract

Register a name for an existing contract that wasn't deployed through the registry:

```bash
stellar registry register-contract \
  --contract-name <NAME> \
  --contract-address <CONTRACT_ADDRESS> \
  [--owner <OWNER_ADDRESS>] \
  [--dry-run]
```

Options:

- `--contract-name`: Name to register for the contract, supports prefix notation like `unverified/my-contract` (required)
- `--contract-address`: The contract address to register (required)
- `--owner`: Owner of the contract registration (optional, defaults to source account)
- `--dry-run`: Simulate the operation without executing (optional)

This allows you to add existing contracts to the registry for name resolution without redeploying them.

**Note:** For the root registry, the governance process ([see above](#quick-links)) must approve initial publishes. For the unverified registry, use the `unverified/` prefix.

### Publish Hash

Publish an already-uploaded Wasm hash to the registry. This is useful when you've already uploaded a Wasm binary using `stellar contract upload` and want to register it in the registry:

```bash
stellar registry publish-hash \
  --wasm-hash <HASH> \
  --wasm-name <NAME> \
  --version <VERSION> \
  [--author <AUTHOR_ADDRESS>] \
  [--dry-run]
```

Options:

- `--wasm-hash`: The hex-encoded 32-byte hash of the already-uploaded Wasm (required)
- `--wasm-name`: Name for the published contract, supports prefix notation like `unverified/my-contract` (required)
- `--version`: Version string, e.g., "1.0.0" (required)
- `--author (-a)`: Author address (optional, defaults to source account)
- `--dry-run`: Simulate the operation without executing (optional)

**Note:** For the root registry, the governance process ([see above](#quick-links)) must approve initial publishes. For the unverified registry, use the `unverified/` prefix.

### Fetch Contract ID

Look up the contract ID of a deployed contract by its registered name:

```bash
stellar registry fetch-contract-id <CONTRACT_NAME>
```

Options:

- `CONTRACT_NAME`: Name of the deployed contract, supports prefix notation like `unverified/my-contract` (required)

### Create Contract Alias

A common pattern is `fetch-contract-id` (see above) and then using `stellar contract alias` to create a local alias to the named contract. `create-alias` does that in one command:

```bash
stellar registry create-alias <CONTRACT_NAME> [LOCAL_NAME]
```

Example:

```bash
stellar registry create-alias circle/usdc
```

This creates contract alias `usdc` (without the `circle` prefix), which you can then use to transfer assets:

```bash
stellar contract invoke --id usdc -- transfer \
    --from account-1 \ # as created with `stellar keys`
    --to account-2 \
    --amount 10000000 # 1 USDC; always check an asset's `decimals` before sending
```

:::caution Careful with SACs!

The `usdc` transfer example above uses `contract invoke`, which uses the Soroban wrapper/interface for the USDC token. This Soroban wrapper/interface is called a Stellar Asset Contract (SAC). Not all legacy systems notice transfers made via SAC. Always test transfers with small amounts and verify your target system works as expected. If you need to use a Stellar Classic transaction, the equivalent to the above would be:

```bash
stellar tx new payment --asset USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN \
    --source account-1 \
    --destination account-2 \
    --amount 10000000
```

:::

Options:

- `CONTRACT_NAME`: Name of the deployed contract, supports prefix notation like `unverified/my-contract` (required).
- `LOCAL_NAME`: Optional custom local name for the alias. If not provided, uses the name from the registry.
- `--force` / `-f`: Force overwrite if an alias with the same name already exists, and allow aliasing a contract flagged as compromised in the registry.

### Fetch Hash

Fetch the Wasm hash of a published contract:

```bash
stellar registry fetch-hash <WASM_NAME> [--version <VERSION>]
```

Options:

- `WASM_NAME`: Name of the published Wasm, supports prefix notation like `unverified/my-contract` (required)
- `--version`: Specific version to fetch (optional, defaults to latest version)

### Current Version

Get the current (latest) version of a published Wasm:

```bash
stellar registry current-version <WASM_NAME>
```

Options:

- `WASM_NAME`: Name of the published Wasm, supports prefix notation like `unverified/my-contract` (required)

### Fetch Contract Owner

Look up the owner who registered a contract name:

```bash
stellar contract invoke --id <REGISTRY_CONTRACT_ID> -- \
  fetch_contract_owner \
  --contract-name <NAME>
```

## Configuration

The registry CLI respects the following environment variables:

- `STELLAR_REGISTRY_CONTRACT_ID`: Override the default registry contract ID
- `STELLAR_NETWORK`: Network to use (e.g., "testnet", "mainnet")
- `STELLAR_RPC_URL`: Custom RPC endpoint (default: https://soroban-testnet.stellar.org:443)
- `STELLAR_NETWORK_PASSPHRASE`: Network passphrase (default: Test SDF Network ; September 2015)
- `STELLAR_ACCOUNT`: Source account to use

These variables can also be in a `.env` file in the current working directory if your shell is configured to respect `.env`.

You can also configure `stellar-cli` defaults:

```bash
stellar keys use alice
stellar network use testnet
```

## Example Workflow

### Using the Unverified Registry

For most users, the unverified registry allows publishing without manager approval:

#### 1. Publish a contract to the unverified registry:

```bash
stellar registry publish \
  --wasm path/to/token.wasm \
  --wasm-name unverified/my-token \
  --binver "1.0.0"
```

#### 2. Register an already-uploaded Wasm with the unverified registry:

```bash
stellar registry publish-hash \
  --wasm-hash d1d4e69… \
  --wasm-name unverified/my-token \
  --version "1.0.0"
```

#### 3. Deploy a contract with constructor arguments from an on-Registry Wasm:

```bash
stellar registry deploy \
  --contract-name unverified/my-token \
  --wasm-name oz/ft-standard \
  --version "1.0.0" \
  -- \
  --name "My Token" \
  --symbol "MTK" \
  --decimals 7 \
  --owner G123… \
  --initial_supply 100
```

#### 4. Register an already-deployed Contract with the unverified registry:

```bash
stellar registry register-contract \
  --contract-name <NAME> \
  --contract-address <CONTRACT_ADDRESS> \
```

### Using a Verified Registry

A verified registry, such as the root registry, requires manager approval for initial publishes & deploys. Use the governance process [described above](#quick-links) to get your contract approved for publication.

## Best Practices

1. Use descriptive contract and wasm names that reflect the contract's purpose
2. Follow semantic versioning for your contract versions
3. Always test deployments on testnet before mainnet
4. Use the `--dry-run` flag to simulate operations before executing them
5. Document initialization parameters used for each deployment
6. Use explicit `--source` and `--network` settings in every command, rather than relying on `stellar network use` and `stellar keys use` settings

## Troubleshooting

### Common Issues

1. **Contract name already exists**: Contract names must be unique within each registry. Choose a different name or check if you own the existing contract.

2. **Version must be greater than current**: When publishing updates, ensure the new version follows semantic versioning and is greater than the currently published version.

3. **Authentication errors**: Ensure your source account has sufficient XLM balance and is properly configured.

4. **Network configuration**: Verify your network settings match the intended deployment target (testnet vs mainnet).

5. **Manager approval required**: For the verified registry, initial publishes and contract name registrations require manager approval (see "governance" section [above](#quick-links)). Use the `unverified/` prefix to publish without approval.

6. **Invalid name**: Names must start with an alphabetic character and contain only alphanumeric characters, hyphens, or underscores. Rust keywords cannot be used as names.

For more detailed information about the available commands:

```bash
stellar registry --help
stellar registry <command> --help
```
