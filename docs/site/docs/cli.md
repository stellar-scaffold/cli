# CLI Commands

Scaffold Stellar provides several CLI commands to help manage your Stellar smart contract development.

## Init Command

Initialize a new Scaffold Stellar project:

```bash
stellar scaffold init <project-path> [name]
```

Options:

- `project-path`: Required. The path where the project will be created

The init command creates:

- A new Stellar smart contract project with best practices and configurations
- A frontend application using the [scaffold-stellar-frontend](https://github.com/stellar-scaffold/ui) template
- Configuration files for both contract and frontend development

## Generate Command

Generate a new contract from examples or wizard. Have a look at their official [documentation](https://docs.openzeppelin.com/stellar-contracts).

```bash
stellar scaffold generate contract [options]
```

Options:

- `--from`: Clone contract from `OpenZeppelin` examples
- `--ls`: List available contract examples
- `--from-wizard`: Open contract generation wizard in browser
- `-o <output>` or `--output <output>`: Output directory for the generated contract (defaults to `contracts/<example-name>`)

The generate command:

- Downloads an `OpenZeppelin` contract from https://github.com/OpenZeppelin/stellar-contracts

## Upgrade Command

Transform an existing Soroban workspace into a full scaffold project:

```bash
stellar scaffold upgrade [workspace-path]
```

Options:

- `workspace-path`: Path to existing workspace (defaults to current directory)

The upgrade command:

- Validates the existing workspace (requires `Cargo.toml` and `contracts/` directory)
- Downloads and integrates the frontend template
- Generates `environments.toml` with discovered contracts
- Analyzes contracts for constructor arguments and prompts for configuration
- Preserves all existing contract code and project structure
- Adds development tools and configurations

Requirements for upgrade:

- Must have a `Cargo.toml` file in the workspace root
- Must have a `contracts/` directory with Soroban contracts
- Contracts should be properly configured as `cdylib` crates

## Build Command

Build contracts and generate frontend client packages:

```bash
stellar scaffold build [options]
```

Options:

- `--build-clients`: Generate TypeScript client packages for contracts
- `--list` or `--ls`: List package names in order of build
- [Standard Soroban contract build options also supported]

## Dev Command

Start development mode with hot reloading:

```bash
stellar scaffold watch [options]
```

Options:

- `--build-clients`: Generate TypeScript client packages while watching
- All options from the build command are also supported

## Update Environment Command

Update environment variables in the .env file:

```bash
stellar scaffold update-env --name <var-name> [options]
```

Options:

- `--name`: Name of environment variable to update
- `--value`: New value (if not provided, reads from stdin)
- `--env-file`: Path to .env file (defaults to ".env")
