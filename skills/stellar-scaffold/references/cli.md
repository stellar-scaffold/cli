# `stellar scaffold` command reference

Run every command as `stellar scaffold <command>`. All commands accept the Stellar CLI global flags: `--config-dir`, `-q/--quiet`, `-v/--verbose`, `--very-verbose`, `-f/--filter-logs`, `--no-cache`.

Network selection, wherever a command takes `--network`: the flag, else `STELLAR_NETWORK`, else the `stellar network use` default, else `local`. The name must be declared under `networks:` in `scaffold.yml`.

## Projects

| Command | Purpose | Key arguments |
|---|---|---|
| `init <PROJECT_PATH>` | Create a project from the official template monorepo, or a community repo | `--template <react\|svelte\|none\|org/repo[#ref]>`, `--no-template` (same as `none`), `-p/--package-manager <npm\|pnpm\|yarn\|bun\|deno>`, `-y/--yes` |
| `upgrade [WORKSPACE_PATH]` | Add a frontend to an existing Soroban workspace (needs `Cargo.toml` and `contracts/`) | `--template <react\|svelte>`, `--skip-prompt` (don't ask for constructor arguments) |
| `generate contract` | Add a contract crate to `contracts/` | `--from <oz/NAME\|stellar/NAME>`, `--ls`, `--from-wizard`, `-o/--output <DIR>` (default `contracts/<example>`), `--force` |
| `update-env --name <NAME>` | Set one variable in `.env` | `--value <VALUE>` (else read from stdin), `--env-file <PATH>` (default `.env`) |
| `version` | Print the plugin version | |

`init` without `-y` or `--template` prompts for a framework and a package manager. With `--template none` it creates a contracts-only workspace with no `app/`, no `app-lib/`, and no JS dependencies.

`upgrade` copies template files without overwriting anything that exists, then writes a legacy `environments.toml`. Fix `scaffold.yml` afterwards as described in the main skill.

`generate contract --from` picks from two example sets, each pinned to a release this CLI was tested with: `oz/<name>` ([OpenZeppelin stellar-contracts](https://github.com/OpenZeppelin/stellar-contracts) examples) and `stellar/<name>` (soroban-examples). It does not edit `scaffold.yml`.

## Building

| Command | Purpose | Key arguments |
|---|---|---|
| `build` | Compile every `cdylib` crate in dependency order | `--build-clients` (also deploy and generate clients for the selected network), `--network <NAME>`, `--package <NAME>`, `--manifest-path <PATH>`, `--profile <NAME>` (default `release`), `--out-dir <DIR>`, `--features`, `--all-features`, `--no-default-features`, `--locked`, `--optimize[=false]`, `--meta <KEY=VALUE>`, `--list/--ls`, `--print-commands-only` |
| `watch` | `build`, then rebuild on any change to a contract or `scaffold.yml` | Same as `build`. The project's `npm run dev` runs `stellar scaffold watch --build-clients` |
| `clean` | Remove Scaffold-generated artifacts | `--manifest-path <PATH>` |

Without `--build-clients`, `build` only compiles. With it, for the selected network it also: starts the local container (when the network's passphrase is the standalone one, or `start-container: true`), creates and funds the network's `accounts`, deploys or upgrades `workspace` contracts whose Wasm changed, runs their `after-deploy` methods, and regenerates `<clients-dir>/<name>/` and `<clients-dir>/index.ts`. If one contract fails, the rest still deploy and export, then the build exits non-zero.

`build` and `watch` also accept a positional `[ENV]` (`development`, `testing`, `staging`, `production`, or `STELLAR_SCAFFOLD_ENV`). It only applies to legacy projects with `environments.toml`, and their `--help` text still describes that legacy flow.

`clean` deletes `target/stellar/`, everything generated in the clients directory (keeping checked-in files like `.gitkeep`), and the contract and identity aliases Scaffold created for the local and test networks. Use it to force fresh deployments, then rerun `build --build-clients` or `npm run dev`.

## Config and diagnostics

| Command | Purpose | Key arguments |
|---|---|---|
| `config check` | Validate `scaffold.yml`; exit non-zero on errors | `--network <NAME>` (also check that it's declared), `--strict` (fail on warnings), `--json`, `--manifest-path` |
| `config show` | Print the fully resolved config for one network | `--network <NAME>` (default `STELLAR_NETWORK`, then `local`), `--json` (else YAML), `--manifest-path` |
| `doctor` | Check toolchain, project, and local network | `--json`, `--strict`, `--manifest-path`, `--env <ENV>` (legacy projects only) |
| `ext ls` | List configured extensions and their hooks | `[ENV]` (legacy projects only) |

`config check` makes no network calls and reads no keys, so it is safe in CI. Each diagnostic has a code (e.g. `unknown-key`, `crate-not-found`, `unknown-account`, `deploy-not-allowed`), the line, and a suggested fix. `--json` prints `[]` when clean.

`config show` applies `extends`, fills in built-in RPC URLs and passphrases, merges per-network contract overrides, and substitutes `${env.…}` and `${network.…}`. `${account.…}` stays symbolic. Contracts that don't list the network are omitted.

`doctor` checks `rustc` (against `rust-toolchain.toml`), the `wasm32v1-none` target, the `stellar` CLI version, Node and the package manager named by `project.package-manager` (or a legacy `package.json` `packageManager` pin), `scaffold.yml` (listing every `config check` problem), the `contracts-dir` and `clients-dir` directories, the project's `engines.stellar-scaffold` constraint, and `.env`. Its Docker, local RPC, and extension checks currently run only for legacy `environments.toml` projects. Exit code is `1` on any error, or on warnings with `--strict`.

## Examples

```bash
# New React project, no prompts, pnpm
stellar scaffold init my-app --template react -p pnpm -y

# Contracts-only project
stellar scaffold init my-contracts --template none

# Build and deploy for testnet once, without the dev server
stellar scaffold build --build-clients --network testnet

# Validate in CI
stellar scaffold config check --strict --json
stellar scaffold doctor --strict --json

# Start over with fresh local deployments
stellar scaffold clean && npm run dev
```
