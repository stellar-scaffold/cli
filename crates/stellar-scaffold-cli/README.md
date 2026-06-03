# stellar-scaffold-cli

CLI toolkit for Stellar smart contract development, providing project scaffolding, build automation, and development workflow tools.

Stellar Scaffold CLI comes with four main commands:

* `stellar scaffold init` - Creates a new Stellar smart contract project with best practices and configurations in place, including an `environments.toml` file for managing network settings, accounts, and contracts across different environments.

* `stellar scaffold upgrade` - Transforms an existing Soroban workspace into a full scaffold project by adding frontend components, environment configurations, and project structure. Preserves existing contracts while adding the complete development toolkit.

* `stellar scaffold build` - Manages two key build processes:
  * Build smart contracts with metadata and handle dependencies
  * Generate TypeScript client packages for frontend integration
  
  The build process respects environment configurations from `environments.toml` and handles contract deployment states based on the current environment (controlled via `STELLAR_SCAFFOLD_ENV`).

* `stellar scaffold watch` - Development mode that monitors contract source files and `environments.toml` for changes, automatically rebuilding as needed. Defaults to using the `development` environment.

## Getting Started

### New Project

1. Install the CLI:
```bash
cargo install --locked stellar-scaffold-cli
```

Or [`cargo-binstall`](github.com/cargo-bins/cargo-binstall):

```bash
cargo binstall --locked stellar-scaffold-cli
```

2. Create a new project:
```bash
stellar scaffold init my-project
cd my-project
```

This creates:
- A smart contract project with recommended configurations
- A frontend application based on [stellar-scaffold/ui](https://github.com/stellar-scaffold/ui)
- Environment configurations for both contract and frontend development

### Upgrading Existing Workspace

If you have an existing Soroban workspace, you can upgrade it to a full scaffold project:
```bash
cd my-existing-workspace
stellar scaffold upgrade
```

This will:
- Add the frontend application and development tools
- Generate `environments.toml` with your existing contracts
- Set up environment files and configurations
- Preserve all your existing contract code and structure

3. Set up your environment:
```bash
cp .env.example .env
```

4. Start development:
```bash
stellar scaffold watch --build-clients
```

## Environment Configuration

Projects use `environments.toml` to define network settings, accounts, and contract configurations for different environments. Example:
```toml
[development]
network = { 
    name = "local",
    run_locally = true
}
accounts = ["account1", "account2"]

[staging]
network = { 
    name = "testnet"
}

[production]
network = { 
    name = "mainnet"
}
```

## Build Process Details

`stellar scaffold build` and `stellar scaffold watch` manage:

1. Smart contract compilation and deployment based on environment
2. TypeScript client package generation for frontend integration
3. Network and account management (create/fund accounts in development)
4. Contract initialization via constructor args and post-deploy scripts

The build process ensures:
- Correct dependency resolution and build order
- Environment-specific contract deployments
- TypeScript client generation for frontend integration
- Contract state verification and updates

### Setting contract metadata

Contract metadata is set when running `stellar scaffold build`. You can configure metadata with 
`[package.metadata.stellar]` section in your `Cargo.toml` file.
For example:
```toml
[package.metadata.stellar]
# When set to `true` will copy over [package] section's `name`, `authors`, `homepage` (renamed to `home_domain` to comply with SEP-47), `repository` (renamed to `source_repo` to comply with SEP-47) and `version` (renamed to `binver` to comply with SEP-47)
cargo_inherit = true
# Override one of the inherited values
name = "my-awesome-contract"
homepage = "theaha.co"
repository = "https://github.com/your-org/your-project"
```

## Environment Variables

- `STELLAR_SCAFFOLD_ENV`: Sets current environment (development/staging/production)
- `STELLAR_ACCOUNT`: Default account for transactions
- `STELLAR_RPC_URL`: RPC endpoint URL
- `STELLAR_NETWORK_PASSPHRASE`: Network passphrase

## For More Information

See the full documentation:
- [CLI Commands Guide](https://scaffoldstellar.com/docs/cli)
- [Environment Configuration](https://scaffoldstellar.com/docs/environments)
- [Deployment Guide](https://scaffoldstellar.com/docs/deploy)