# Registry Guide

The Stellar Registry is a system for publishing, deploying, and managing smart contracts on the Stellar network. This guide explains how to use the registry CLI tools to manage your contracts.

## Overview

The registry system consists of two main components:

1. **On-chain registry contracts** - A root "verified" registry and an "unverified" registry
2. The `stellar-registry` CLI tool for interacting with the registries

### Registry Types

There are two types of registries:

- **Verified (Root) Registry** - A managed registry where a manager account must approve initial publishes and contract name registrations. This ensures that established contracts in the verified registry have been vetted. If interested in having your contract added to the registry please open an issue and we can begin to verify your contract and project.
- **Unverified Registry** - An unmanaged registry where anyone can publish wasms or register contract names without approval.

### Name Resolution

Names in the registry support namespace prefixes. The CLI resolves names using the root registry as the source of truth:

- `my-contract` - Looks up in the verified (root) registry
- `unverified/my-contract` - First fetches the `unverified` registry contract ID from the root registry, then looks up `my-contract` in that registry

### Name Normalization

All names are normalized before storage:

- Underscores (`_`) are converted to hyphens (`-`)
- Uppercase letters are converted to lowercase
- Names must start with an alphabetic character
- Names can only contain alphanumeric characters, hyphens, or underscores
- Rust keywords are not allowed as names
- Names have a maximum length of 64 characters

## Prerequisites

- Install the registry CLI:

```bash
cargo install --git https://github.com/stellar-registry/cli stellar-registry-cli
```

## Commands

### Publish Contract

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

**Note:** For the verified registry, the manager must approve initial publishes. For the unverified registry, use the `unverified/` prefix.

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
- `CONSTRUCTOR_ARGS`: Optional arguments for the constructor function

Note: Use `--` to separate CLI options from constructor arguments.

**Note:** For the verified registry, the manager must approve deploying with a registered name. For the unverified registry, use the `unverified/` prefix.

### Deploy Unnamed Contract

Deploy a published contract without registering a name in the registry. This is useful when you want to deploy a contract but don't need name resolution:

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

**Note:** For the verified registry, the manager must approve name registrations. Use the `unverified/` prefix for the unverified registry.

### Install Contract

Install a deployed contract as an alias to be used by `stellar-cli`:

```bash
stellar registry create-alias <CONTRACT_NAME>
```

Options:

- `CONTRACT_NAME`: Name of the deployed contract to install, supports prefix notation like `unverified/my-contract` (required)

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

### Fetch Contract ID

Look up the contract ID of a deployed contract by its registered name:

```bash
stellar registry fetch-contract-id <CONTRACT_NAME>
```

Options:

- `CONTRACT_NAME`: Name of the deployed contract, supports prefix notation like `unverified/my-contract` (required)

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

These variables can also be in a `.env` file in the current working directory.

You can also configure `stellar-cli` defaults:

```bash
stellar keys use alice
stellar network use testnet
```

## Example Workflow

### Publishing to the Unverified Registry

For most users, the unverified registry allows publishing without manager approval:

1. Publish a contract to the unverified registry:

```bash
stellar registry publish \
  --wasm path/to/token.wasm \
  --wasm-name unverified/my-token \
  --binver "1.0.0"
```

2. Deploy the published contract with constructor arguments:

```bash
stellar registry deploy \
  --contract-name unverified/my-token-instance \
  --wasm-name unverified/my-token \
  --version "1.0.0" \
  -- \
  --name "My Token" \
  --symbol "MTK" \
  --decimals 7
```

3. Install the deployed contract locally:

```bash
stellar registry create-alias unverified/my-token-instance
```

4. Use the installed contract with `stellar-cli`:

```bash
stellar contract invoke --id my-token-instance -- --help
```

### Publishing to the Verified Registry

The verified registry requires manager approval for initial publishes. Contact the registry manager to get your contract approved for publication.

## Best Practices

1. Use descriptive contract and wasm names that reflect the contract's purpose
2. Follow semantic versioning for your contract versions
3. Always test deployments on testnet before mainnet
4. Use the `--dry-run` flag to simulate operations before executing them
5. Document initialization parameters used for each deployment
6. Use environment variables or `.env` files for different network configurations

## Registry Contract Addresses

The **verified (root) registry** contract is deployed at different addresses for each network:

- **Testnet**: `CBFFTTX7QKA76FS4LHHQG54BC7JF5RMEX4RTNNJ5KEL76LYHVO3E3OEE`
- **Mainnet**: `CCRKU6NT4CRG4TVKLCCJFU7EOSAUBHWGBJF2JWZJSKTJTXCXXTKOJIUS`
- **Futurenet**: `CBUP2U7IY4GBZWILAGFGBOGEJEVSWZ6FAIKAX2L7PYOEE7R556LNXRJM`
- **Local**: `CDUK4O7FPAPZWAMS6PBKM7E4IO5MCBJ2ZPZ6K2GOHK33YW7Q4H7YZ35Z`

The **unverified registry** is deployed by the root registry and can be looked up using:

```bash
stellar contract invoke --id <ROOT_REGISTRY_ID> -- fetch_contract_id --contract-name unverified
```

## Troubleshooting

### Common Issues

1. **Contract name already exists**: Contract names must be unique within each registry. Choose a different name or check if you own the existing contract.

2. **Version must be greater than current**: When publishing updates, ensure the new version follows semantic versioning and is greater than the currently published version.

3. **Authentication errors**: Ensure your source account has sufficient XLM balance and is properly configured.

4. **Network configuration**: Verify your network settings match the intended deployment target (testnet vs mainnet).

5. **Manager approval required**: For the verified registry, initial publishes and contract name registrations require manager approval. Use the `unverified/` prefix to publish without approval.

6. **Invalid name**: Names must start with an alphabetic character and contain only alphanumeric characters, hyphens, or underscores. Rust keywords cannot be used as names.

For more detailed information about the available commands:

```bash
stellar registry --help
stellar registry <command> --help
```
