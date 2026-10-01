---
name: stellar-scaffold
description: Build full-stack Stellar dApps with Stellar Scaffold, the `stellar scaffold` CLI plugin. Use when creating a new Soroban project with a frontend (`stellar scaffold init`), adding a frontend to an existing Soroban workspace (`upgrade`), configuring networks, accounts, and contracts in `scaffold.yml`, running the local dev loop (`npm run dev` / `stellar scaffold watch --build-clients`), calling contracts from a React or Svelte app through the generated clients in `@stellar-scaffold/app-lib/clients`, adding example contracts (`generate contract`), building for testnet or mainnet, or debugging a Scaffold build with `config check` and `doctor`.
---

# Stellar Scaffold

Stellar Scaffold turns a Rust/Soroban contract workspace and a TypeScript frontend into one project:

- **`stellar scaffold` CLI plugin**: creates projects, builds and deploys contracts, and generates a typed TypeScript **client** for each one.
- **`scaffold.yml`**: one config file at the project root that says which networks you build for, which accounts exist on each, and which contracts to deploy or connect to on each network.
- **`@stellar-scaffold/app-lib`**: a workspace package in every project with wallet, network, and formatting helpers, plus the generated clients at `@stellar-scaffold/app-lib/clients`.

Docs: https://stellarscaffold.org/docs

## Setup

Prerequisites: Rust (with the `wasm32v1-none` target), the Stellar CLI (`stellar`), Node.js, and Docker (for the local network).

```bash
cargo install --locked stellar-scaffold-cli   # or: cargo binstall stellar-scaffold-cli
stellar scaffold version
```

`stellar-scaffold` is a plugin: the Stellar CLI finds it on `PATH`, so you always run it as `stellar scaffold <command>`.

## Start a project

```bash
stellar scaffold init my-app --template react -y   # or: --template svelte
cd my-app
npm run dev                                         # app at http://localhost:5173
```

- `--template` takes `react`, `svelte`, `none` (contracts only, no JS at all), or a community repo as `org/repo[#branch]`. Omit it to be prompted. `--no-template` is the same as `--template none`.
- `-p <npm|pnpm|yarn|bun|deno>` picks the package manager. With `--template` or `-y` it defaults to npm without prompting.
- `init` does everything up front: copies `.env.example` to `.env`, installs dependencies, compiles the contracts, deploys them to the local network and generates their clients, then `git init`s and commits. If that build fails (most often because Docker isn't running), `init` only warns and still creates the project. Fix the cause, then run `npm run dev`.

Already have a Soroban workspace (`Cargo.toml` plus `contracts/`)? Run `stellar scaffold upgrade --template react` in it instead. It adds the frontend files and never overwrites files that already exist. **Then fix the config by hand**, because `upgrade` currently copies the template's `scaffold.yml`, whose `contracts:` lists the template's example crates rather than yours, and also writes a legacy `environments.toml` that version 2 ignores:

1. In `scaffold.yml`, replace every entry under `contracts:` with one per crate in your `contracts/` (`type: workspace`, `source: <package name>`, constructor `args`, `networks:`). Use the generated `environments.toml` as a guide to each constructor's arguments.
2. Delete `environments.toml`.
3. Run `stellar scaffold config check` until it reports no errors.

## Project layout

```
my-app/
├── contracts/<crate>/   # Soroban contracts; the workspace builds every cdylib in contracts/*
├── app/                 # your React or Svelte app
├── app-lib/             # @stellar-scaffold/app-lib: wallet, network, format helpers
│   └── clients/         # GENERATED: one package per contract + index.ts. Never edit.
├── scaffold.yml         # networks, accounts, contracts
├── .env                 # STELLAR_NETWORK + PUBLIC_STELLAR_* for the frontend (gitignored)
├── Cargo.toml           # Rust workspace
└── package.json         # npm workspaces root
```

`app/CLAUDE.md` (`app/AGENTS.md` in newer projects) describes the app's own structure: routes, providers, stores. Read it before editing the frontend.

## The dev loop

`npm run dev` runs two processes: `stellar scaffold watch --build-clients` and Vite. On every change to a contract or to `scaffold.yml`, `watch`:

1. Builds the contracts (Wasm in `target/stellar/<STELLAR_NETWORK, default local>/<crate_name>.wasm`, with `-` replaced by `_`).
2. For the selected network: starts the local container if needed, creates and funds the accounts, deploys or upgrades contracts whose Wasm changed, and runs their `after-deploy` calls.
3. Regenerates `app-lib/clients/<name>/` and `app-lib/clients/index.ts`, which Vite hot-reloads.

Changing only constructor `args` or `after-deploy` does **not** redeploy. Run `stellar scaffold clean` and restart to get fresh deployments.

## `scaffold.yml` at a glance

```yaml
version: 2

networks:
  local:
    accounts: [me] # created and funded; the first one signs deploys
  testnet:
    accounts: [testnet-user]
    allow-deploy: true # public networks don't deploy unless you opt in

contracts:
  guess-the-number: # client name: exported as `guessTheNumber`
    type: workspace # build + deploy this crate from contracts/
    source: guess-the-number # the crate's package name
    args: # __constructor arguments, by parameter name
      admin: ${account.me}
    after-deploy: [reset] # no-argument methods called after deploy
    networks: # the contract ONLY exists on networks listed here
      local:
      testnet:
        args:
          admin: ${account.testnet-user} # accounts belong to one network
```

Rules that trip agents up:

- **A contract only exists on the networks it lists.** Building any other network skips it, and its client is not exported, so imports of it fail to typecheck.
- **`build` supports only `type: workspace` and `type: contract`** (a deployed `C…` address) today. `registry`, `asset`, `wasm-file`, `wasm-hash`, `wasm-registry`, `from-network` and `after-deploy-script` pass validation but fail at build time.
- **Keys are kebab-case.** Unknown keys are errors, with a "did you mean" hint.
- **`args` values are typed YAML, not shell.** Use `${account.<name>}` for addresses, `${env.VAR}` for secrets. `$(…)` is literal text.

After every edit, run `stellar scaffold config check`. To see exactly what a build will use, run `stellar scaffold config show --network <name>`. Full schema: [references/config.md](references/config.md).

## Calling contracts from the app

Import the generated client by its camelCase name. Never import from `app-lib/clients/<name>/` directly, and never hardcode a contract ID or RPC URL.

```ts
import { guessTheNumber } from "@stellar-scaffold/app-lib/clients"

// Read: simulate only, no signature
const { result: admin } = await guessTheNumber.admin()

// Write: build with the caller as source, then sign with the connected wallet and send
const tx = await guessTheNumber.guess(
  { a_number: BigInt(5), guesser: address },
  { publicKey: address },
)
const { result } = await tx.signAndSend({ signTransaction })
if (result.isErr()) console.error(result.unwrapErr())
```

Method names and argument names come straight from the Rust contract (`snake_case`), so `i128`/`u64` and similar values are `bigint`. Wallet connection, `signTransaction`, balances, and network info come from `@stellar-scaffold/app-lib`. Details: [references/frontend.md](references/frontend.md).

## Adding a contract

```bash
stellar scaffold generate contract --ls                 # list examples
stellar scaffold generate contract --from oz/fungible-pausable
# or write your own crate under contracts/<name>/ with crate-type = ["cdylib"]
```

`generate` writes `contracts/<example-name>/` and merges its dependencies into the workspace `Cargo.toml`. It does **not** touch `scaffold.yml`, so add an entry yourself:

```yaml
contracts:
  my-token:
    type: workspace
    source: fungible-pausable-example # `name` from contracts/<dir>/Cargo.toml
    args:
      owner: ${account.me}
    networks:
      local:
```

Then `stellar scaffold config check`. The client appears as `myToken` in `@stellar-scaffold/app-lib/clients` on the next build.

## Testnet and mainnet

The network comes from `--network <name>`, else `STELLAR_NETWORK` (set in `.env`), else `stellar network use`, else `local`. Switching networks takes two changes in `.env`:

```bash
STELLAR_NETWORK=testnet                     # what the CLI builds and deploys to
PUBLIC_STELLAR_NETWORK="TESTNET"            # what the frontend connects to
PUBLIC_STELLAR_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
PUBLIC_STELLAR_RPC_URL="https://soroban-testnet.stellar.org"
PUBLIC_STELLAR_HORIZON_URL="https://horizon-testnet.stellar.org"
```

`testnet`, `futurenet` and `mainnet` refuse Scaffold deploys until the network sets `allow-deploy: true`. For production, deploy once yourself and point Scaffold at the result instead of deploying from the dev loop:

```yaml
networks:
  mainnet:
    rpc-url: https://your-rpc-provider.example # stellar-cli has no default mainnet RPC
    rpc-headers:
      Authorization: "Bearer ${env.MAINNET_RPC_TOKEN}"

contracts:
  guess-the-number:
    # ...
    networks:
      mainnet:
        type: contract
        source: CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
```

To publish and deploy by name, use the Stellar Registry: https://raw.githubusercontent.com/stellar-registry/cli/main/skills/stellar-registry/SKILL.md (then `stellar registry fetch-contract-id <name>` gives the `C…` address for `source`). Build the frontend with `npm run build --workspace app` (output in `app/dist/` for React, `app/build/` for Svelte).

## Diagnosing problems

```bash
stellar scaffold config check            # every scaffold.yml problem, with line numbers and fixes
stellar scaffold config show --network local
stellar scaffold doctor                  # toolchain, project, and local network health
```

Full command list: [references/cli.md](references/cli.md).

## Common pitfalls

- **Edited `app-lib/clients/`**: it is regenerated on every build. Wrap a client in your own module under `app/` instead.
- **`Module has no exported member 'myContract'`**: the contract has no entry for the network you built (`networks:` under the contract), the build failed before codegen, or the client name doesn't match. Client `my-contract` exports as `myContract`.
- **Frontend talks to the wrong network**: `STELLAR_NETWORK` and the `PUBLIC_STELLAR_*` values in `.env` disagree. The CLI bakes contract IDs for `STELLAR_NETWORK`; the app connects with `PUBLIC_STELLAR_*`.
- **`would be deployed to testnet, which has deploys disabled`**: add `allow-deploy: true` to that network, or point the contract at an existing deployment with `type: contract`.
- **`account <x> is not listed in network <n>'s accounts`**: each network has its own accounts. Add it to `networks.<n>.accounts`, or override `args` for that network.
- **`uses type: registry, which build does not support yet`**: see the supported types above. Use `type: contract` with the address for now.
- **Changed constructor `args`, nothing happened**: only Wasm changes redeploy. Stop the dev server, `stellar scaffold clean`, restart.
- **`STELLAR_SCAFFOLD_ENV is ignored`**: that variable and the `development`/`staging`/`production` environments belong to the legacy `environments.toml`. Use networks and `STELLAR_NETWORK`.
- **Local deploy fails or hangs**: Docker isn't running. `stellar scaffold doctor` checks the toolchain; `docker ps` and `stellar container start local` check the network.
- **`parse-error: while parsing a flow sequence`**: `${…}` inside `[ ]` or `{ }` must be quoted: `["${account.me}"]`. Block style (`admin: ${account.me}`) needs no quotes.
- **`--tutorial`, `client = true`, `constructor_args`, `run_locally`** show up in old docs and blog posts. None of them exist in `scaffold.yml` version 2.
