---
name: stellar-scaffold
description: Build full-stack Stellar dApps with Stellar Scaffold, the `stellar scaffold` CLI plugin. Use when creating a new Soroban project with a frontend (`stellar scaffold init`), adding a frontend to an existing Soroban workspace (`upgrade`), configuring networks, accounts, and contracts in `scaffold.yml`, running the local dev loop (`npm run dev` / `stellar scaffold watch`), calling contracts from a React or Svelte app through the generated clients in `@stellar-scaffold/app-lib/clients`, adding example contracts (`generate contract`), pointing an app at contracts already deployed on testnet or mainnet, or debugging a Scaffold build with `config check` and `doctor`.
---

# Stellar Scaffold

Stellar Scaffold turns a Rust/Soroban contract workspace and a TypeScript frontend into one project:

- **`stellar scaffold` CLI plugin**: creates projects, builds contracts, resolves each network's contract dependencies, and generates a typed TypeScript **client** for each one.
- **`scaffold.yml`**: one config file at the project root that declares your app's **contract dependencies, per network**: which contracts the app uses on each network and where to get each one (a crate in this workspace, or an existing deployment), plus the networks and accounts involved.
- **`@stellar-scaffold/app-lib`**: a workspace package in every project with wallet, network, and formatting helpers, plus the generated clients at `@stellar-scaffold/app-lib/clients`.

**Scaffold is not a deploy tool.** It deploys only as a convenience on throwaway networks (`local`, by default), so the dev loop has something to call. On shared networks (testnet, futurenet, mainnet), contracts get deployed out of band, normally with Stellar Registry, and `scaffold.yml` just references those deployments. On every network, `build` fetches the contract that is actually there and regenerates its client, so the frontend is typechecked against that network's real interface. Contracts can differ subtly between networks, and a fresh client per network is how you find out. Don't reach for `allow-deploy: true` to "ship" with Scaffold.

Docs: https://stellarscaffold.org/docs

## Setup

Prerequisites: Rust (with the `wasm32v1-none` target) and a C toolchain, the Stellar CLI (`stellar`), Node.js, and Docker (for the local network). On Linux, install the Stellar CLI with `curl -fsSL https://github.com/stellar/stellar-cli/raw/main/install.sh | sh -s -- --install-deps` to also get the system libraries Scaffold needs.

```bash
cargo binstall stellar-scaffold-cli   # pre-built; or: cargo install --locked stellar-scaffold-cli
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
- `init` does everything up front: copies `app/.env.example` to `app/.env`, installs dependencies, compiles the contracts, deploys them to the local network and generates their clients, then `git init`s and commits. If that build fails (most often because Docker isn't running), `init` only warns and still creates the project. Fix the cause, then run `npm run dev`.

Already have a Soroban workspace (`Cargo.toml` plus `contracts/`)? Run `stellar scaffold upgrade --template react` in it instead. It adds the frontend files and never overwrites files that already exist. **Then fix the config by hand**, because `upgrade` currently copies the template's `scaffold.yml`, whose `contracts:` lists the template's example crates rather than yours, and also writes a legacy `environments.toml` that version 2 ignores:

1. In `scaffold.yml`, replace every entry under `contracts:` with one per crate in your `contracts/` (`type: workspace`, `source: <package name>`, constructor `args`, `networks:`). Use the generated `environments.toml` as a guide to each constructor's arguments.
2. Delete `environments.toml`.
3. Run `stellar scaffold config check` until it reports no errors.

## Project layout

```
my-app/
├── contracts/<crate>/   # Soroban contracts; the workspace builds every cdylib in contracts/*
├── app/                 # your React or Svelte app
│   └── .env             # optional STELLAR_NETWORK, secrets for ${env.…} (gitignored)
├── app-lib/             # @stellar-scaffold/app-lib: wallet, network, format helpers
│   └── clients/         # GENERATED: one package per contract + index.ts. Never edit.
├── scaffold.yml         # networks, accounts, contracts
├── .scaffold/           # GENERATED: what this checkout deployed (state.json). Ignores itself.
├── Cargo.toml           # Rust workspace
└── package.json         # npm workspaces root
```

`app/CLAUDE.md` (`app/AGENTS.md` in newer projects) describes the app's own structure: routes, providers, stores. Read it before editing the frontend.

## The dev loop

`npm run dev` runs two processes: `stellar scaffold watch` and Vite. On every change to a contract or to `scaffold.yml`, `watch`:

1. Builds the contracts (Wasm in `target/stellar/<STELLAR_NETWORK, default local>/<crate_name>.wasm`, with `-` replaced by `_`).
2. Resolves each contract dependency for the selected network (default: `local`). On `local`, that means starting the container if needed, creating and funding the accounts, deploying or upgrading `workspace` contracts whose Wasm changed, and running their `after-deploy` calls. On a network with deploys disabled, nothing is deployed: `type: contract` entries are fetched from the address given.
3. Regenerates `app-lib/clients/<name>/` and `app-lib/clients/index.ts` (gitignored), which Vite hot-reloads.

Changing only constructor `args` or `after-deploy` does **not** redeploy. Run `stellar scaffold clean` and restart to get fresh deployments.

## `scaffold.yml` at a glance

```yaml
version: 2

networks:
  local:
    accounts: [me] # created and funded; the first one signs deploys
  testnet: # shared: Scaffold never deploys here; reference deployments made out of band
    accounts: [testnet-user]

contracts:
  guess-the-number: # client name: exported as `guessTheNumber`
    type: workspace # built from contracts/; deployed only on networks that allow it (local)
    source: guess-the-number # the crate's package name
    args: # __constructor arguments, by parameter name (used only where Scaffold deploys)
      admin: ${account.me}
    after-deploy: [reset] # no-argument methods called after a Scaffold deploy
    networks: # networks where the app depends on this contract; no client elsewhere
      local:
      testnet:
        type: contract # already deployed (e.g. via Stellar Registry): fetch it
        source: CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
```

Rules that trip agents up:

- **The app only depends on a contract on the networks it lists.** Building any other network skips it, and its client is not exported, so imports of it fail to typecheck.
- **`workspace` on a shared network is an error** (`deploy-not-allowed`). That's deliberate: override the network with `type: contract` pointing at a deployment you made out of band.
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

The network comes from `--network <name>`, else `STELLAR_NETWORK` (from the shell or `.env`), else `stellar network use`, else `local`. The frontend reads its network from the generated clients, so switching is one setting:

```bash
STELLAR_NETWORK=testnet npm run dev
```

Local is for thrashing: Scaffold deploys and redeploys there freely. Testnet is staging, a dry run for mainnet, and on both of those Scaffold only **consumes** deployments. It doesn't make them. Going to a shared network:

1. **Set the network**: `STELLAR_NETWORK=testnet` in `app/.env`.
2. **Check each contract's `Cargo.toml` metadata** (`authors`, `homepage`, `repository`, version; see `[package.metadata.stellar]` in [references/config.md](references/config.md)). It is written into the Wasm and is permanent once published.
3. **Compile for that network**: `stellar scaffold build --compile-only` (no deploys, no clients). The network matters: contracts that use `stellar_registry::import_contract!` for cross-contract calls emit different Wasm per network. Prefer this over `stellar contract build`, which doesn't build crates in dependency order the way `stellar scaffold build` does. If your version lacks `--compile-only`, upgrade the plugin.
4. **Publish and deploy out of band** with Stellar Registry: https://raw.githubusercontent.com/stellar-registry/cli/main/skills/stellar-registry/SKILL.md. First publishes go under `unverified/`.
5. **Reference the deployment** in `scaffold.yml`: under the contract's `networks:`, override that network with `type: contract` and `source: <C… address>` (`stellar registry fetch-contract-id <name>` prints it). `type: registry` with a name is planned but not yet supported by `build`.
6. **Build for that network to typecheck and build the frontend**: `stellar scaffold build --network testnet` deploys nothing. It fetches each referenced contract and regenerates its client, so that `npm run build --workspace app` typechecks and builds the frontend against what is actually deployed (output in `app/dist/` for React, `app/build/` for Svelte). Type errors here usually mean the deployed contract differs from your local one.

Mainnet works the same way, and also needs an RPC provider:

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

`testnet`, `futurenet` and `mainnet` refuse Scaffold deploys unless the network sets `allow-deploy: true`. Don't turn that on to ship. If you truly need disposable deploys on testnet (say, a CI smoke test), add a separate network (`testnet-dev: { extends: testnet, allow-deploy: true }`) so `testnet` itself stays reference-only.

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
- **Built for an unexpected network**: the first line of build output names the network and where it came from. Check `STELLAR_NETWORK` in your shell and `app/.env`.
- **`would be deployed to testnet, which has deploys disabled`**: working as intended. Deploy out of band (Stellar Registry), then point the contract at that deployment with `type: contract`. Don't opt `testnet` into `allow-deploy`.
- **App works locally, breaks on testnet**: the deployed contract differs from your local build. Rerun `stellar scaffold build --network testnet` and fix the type errors it surfaces in the frontend.
- **`account <x> is not listed in network <n>'s accounts`**: each network has its own accounts. Add it to `networks.<n>.accounts`, or override `args` for that network.
- **`uses type: registry, which build does not support yet`**: see the supported types above. Use `type: contract` with the address for now.
- **Changed constructor `args`, nothing happened**: only Wasm changes redeploy. Stop the dev server, `stellar scaffold clean`, restart.
- **`STELLAR_SCAFFOLD_ENV is ignored`**: that variable and the `development`/`staging`/`production` environments belong to the legacy `environments.toml`. Use networks and `STELLAR_NETWORK`.
- **Local deploy fails or hangs**: Docker isn't running. `stellar scaffold doctor` checks the toolchain; `docker ps` and `stellar container start local` check the network.
- **`parse-error: while parsing a flow sequence`**: `${…}` inside `[ ]` or `{ }` must be quoted: `["${account.me}"]`. Block style (`admin: ${account.me}`) needs no quotes.
- **`--tutorial`, `client = true`, `constructor_args`, `run_locally`** show up in old docs and blog posts. None of them exist in `scaffold.yml` version 2.
