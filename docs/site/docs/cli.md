# CLI Commands

Stellar Scaffold provides several CLI commands to help manage your Stellar smart contract development.

## Init Command

Initialize a new Stellar Scaffold project:

```bash
stellar scaffold init <project-path>
```

Options:

- `project-path`: Required. The path where the project will be created
- `--template <name>`: Template selector. A bare framework name (`react` or `svelte`) picks an official template; a `user/repo` shorthand (optionally with `#branch` or `#tag`) degits that community repo directly; `none` creates a contracts-only project with no frontend. Omit to choose interactively
- `--no-template`: Create a contracts-only project with no frontend (alias for `--template none`)
- `--tutorial`: Start from the simplified project the [tutorial](./tutorial/00-overview.md) builds on, rather than a full template
- `-p <name>` or `--package-manager <name>`: Package manager to use. Omit to choose interactively
- `-y` or `--yes`: Accept all defaults and skip interactive prompts

`--tutorial` picks the frontend and the package manager for you, so it does not prompt. Pass `--package-manager` alongside it if you want something other than npm.

The init command creates:

- A new Stellar smart contract project with best practices and configurations
- A frontend application in `app/`, in the framework you chose
- `app-lib/`, utility code your app can use for wallet connection, network settings, and formatting, plus the directory your generated contract clients are written to
- Configuration files for both contract and frontend development

Official templates come from the [Stellar Scaffold templates repo](https://github.com/stellar-scaffold/ui).

With `--no-template` (or `--template none`), the frontend layer is omitted entirely: no `app/`, no JS workspaces, no dependency install. Every contract in `environments.toml` is written with `client = false`, so `stellar scaffold build` and `watch` skip client package generation. Flip a contract back to `client = true` if you later need TypeScript clients.

## Generate Command

Generate a new contract from examples or wizard. Have a look at their official [documentation](https://docs.openzeppelin.com/stellar-contracts).

```bash
stellar scaffold generate contract [options]
```

Options:

- `--from <example>`: Clone a contract from one of two example sets, selected by prefix:
  - `oz/<name>` — an [OpenZeppelin stellar-contracts](https://github.com/OpenZeppelin/stellar-contracts) example, e.g. `--from oz/nft-royalties`
  - `stellar/<name>` — a [soroban-examples](https://github.com/stellar-scaffold/soroban-examples) contract, e.g. `--from stellar/hello-world`
- `--ls`: List available contract examples
- `--from-wizard`: Open the OpenZeppelin Contract Wizard in your browser
- `-o <output>` or `--output <output>`: Output directory for the generated contract (defaults to `contracts/<example-name>`)
- `--force`: Overwrite the destination path if it already exists

The generate command downloads the example, caches it locally, writes it to `contracts/<example-name>/`, and merges the dependencies it needs into your workspace `Cargo.toml`. Each example set is pinned to a supported release rather than tracking upstream `main`, so the version you get is the one this CLI release was tested against.

If the contract takes constructor arguments, add them to `environments.toml` yourself — `generate` does not write them for you.

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

## Clean Command

Remove the artifacts Stellar Scaffold generated for a project:

```bash
stellar scaffold clean [options]
```

Options:

- `--manifest-path <path>`: Path to `Cargo.toml` (defaults to the current directory)

Clean removes:

- `target/stellar/`, the deployment state for the local and test networks
- Everything generated into your clients directory, while leaving checked-in files such as `.gitkeep` alone
- The contract and identity aliases the CLI created for the local and test networks

Reach for it when you want a genuinely fresh deployment. Because Stellar Scaffold only redeploys a contract when the contract itself changes, editing something else — an `after_deploy` script, for instance — will not produce a new instance on its own. Clearing the aliases makes the next `stellar scaffold watch` deploy from scratch.

## Update Environment Command

Update environment variables in the .env file:

```bash
stellar scaffold update-env --name <var-name> [options]
```

Options:

- `--name`: Name of environment variable to update
- `--value`: New value (if not provided, reads from stdin)
- `--env-file`: Path to .env file (defaults to ".env")
