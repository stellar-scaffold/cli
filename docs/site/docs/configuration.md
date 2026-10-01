# Configuration

Every Stellar Scaffold project is configured by one file at its root: `scaffold.yml`. It says where your contracts and generated clients live, which Stellar networks you build for, which accounts exist on each, and which contracts to deploy or connect to on each network.

```yaml
version: 2

project:
  contracts-dir: contracts
  clients-dir: app-lib/clients

networks:
  local:
    accounts: [me]
  testnet:
    accounts: [testnet-user]
    allow-deploy: true

contracts:
  guess-the-number:
    type: workspace
    source: guess-the-number
    args:
      admin: ${account.me}
    after-deploy: [reset]
    networks:
      local:
      testnet:
        args:
          admin: ${account.testnet-user}
```

Machine-local settings and secrets stay out of this file. They go in `.env`, which is gitignored and also read by your frontend.

:::note Projects created before `scaffold.yml` version 2 use `environments.toml` instead. `build` and `watch` still read it as long as `scaffold.yml` says `version: 1`. See [Legacy: `environments.toml`](#legacy-environmentstoml) below. :::

## Choosing a network

`build` and `watch` work against one network at a time. They pick it in this order:

1. `--network <name>`, or the `STELLAR_NETWORK` environment variable
2. The default set with `stellar network use <name>`
3. `local`

The name must be declared under `networks:`. New projects set `STELLAR_NETWORK=local` in `.env`; switch it to `testnet` to build against testnet. Keep the `PUBLIC_STELLAR_*` values in `.env` pointed at the same network, since those are what your frontend connects to.

`STELLAR_SCAFFOLD_ENV` is ignored with a version 2 `scaffold.yml`. The CLI warns if it is set.

## `version`

Required, and must be `2`. The CLI checks it before anything else, so a file written for a different schema version gets one clear error instead of a list of unknown keys.

## `project`

Where things live. Both keys are optional.

| Key | Default | Meaning |
| --- | --- | --- |
| `contracts-dir` | `contracts` | Where your Rust contract crates live |
| `clients-dir` | `app-lib/clients` | Where generated clients are written: one package per contract in `<clients-dir>/<name>/`, plus `<clients-dir>/index.ts`, which your app imports as `@stellar-scaffold/app-lib/clients` |

Everything in `clients-dir` is regenerated on each build, so hand edits are overwritten. To customize a client, import it into your own code instead.

## `networks`

Each key is a network name you choose. Names may contain letters, digits, `-` and `_`, and must start with a letter or digit.

```yaml
networks:
  local:
    accounts: [me, alice]
    default-account: me

  testnet:
    accounts: [testnet-user]
    allow-deploy: true

  # A second testnet setup with its own accounts, inheriting testnet's RPC
  preview:
    extends: testnet
    accounts: [preview-user]

  mainnet:
    rpc-url: https://your-rpc-provider.example
    rpc-headers:
      Authorization: "Bearer ${env.MAINNET_RPC_TOKEN}"
    accounts: [mainnet-user]
```

A network with no settings can be written as a bare `testnet:`, which is the same as `testnet: {}`.

| Key | Default | Meaning |
| --- | --- | --- |
| `rpc-url` | stellar-cli's URL for `local`, `testnet` and `futurenet` | RPC endpoint. Required for `mainnet` and for any custom name, unless inherited through `extends` |
| `network-passphrase` | stellar-cli's passphrase for the built-in names | Required for custom names, unless inherited |
| `rpc-headers` | none | Map of HTTP headers sent with every RPC request |
| `accounts` | none | Account aliases Scaffold creates on this network, funding them where it can (Friendbot or the local container) |
| `default-account` | first entry in `accounts` | Which account signs deploys when a contract sets no `signer`. Must be listed in `accounts` |
| `start-container` | `true` only for the local (standalone) passphrase | Whether `build` starts a `stellar/quickstart` container. Set `false` if a local chain is already running at `rpc-url` |
| `allow-deploy` | `false` for the testnet, futurenet and mainnet passphrases; `true` otherwise | Whether Scaffold may deploy `workspace` contracts here |
| `extends` | none | Another declared network to inherit settings from |

`allow-deploy` and `start-container` defaults come from the network's passphrase, not its name. A custom network pointed at testnet is treated like testnet, and a custom network with the standalone passphrase gets a local container.

### `extends`

A network that `extends` another starts from the parent's resolved settings and applies its own on top. Scalars override, maps (such as `rpc-headers`) merge, and lists (such as `accounts`) replace rather than append. An explicit `allow-deploy` is inherited; an unset one is recomputed from the child's own passphrase. Chains may be at most three hops deep and may not loop.

Use `extends` when you need a second configuration on the same chain, for example a `preview` deployment on testnet with its own accounts.

### Interpolation in network settings

`rpc-url`, `network-passphrase` and `rpc-headers` values may use `${env.NAME}` to read an environment variable at build time, so secrets such as RPC tokens stay in `.env`. See [Interpolation](#interpolation).

## `contracts`

Each key is the client name: the name of the generated package in `<clients-dir>/<name>/` and, in camelCase, the name exported from `index.ts` (`guess-the-number` becomes `guessTheNumber`). Client names are independent of crate names.

```yaml
contracts:
  my-token:
    type: workspace
    source: fungible-token # the crate's package name
    signer: admin
    args:
      owner: ${account.admin}
      initial_supply: 1000000000000000000000000
    after-deploy: [unpause]
    networks:
      local:
      testnet:
        args:
          owner: ${account.testnet-admin}
        signer: testnet-admin
      mainnet:
        type: contract
        source: CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
```

Keys directly under a contract are defaults for every network it lists. Entries under its `networks:` override them, using the same merge rules as `extends`. A per-network entry that changes `type` must also set `source`, and the other way round.

**A contract only exists on the networks it lists.** Building any other network skips it: it is not built, not deployed, and not exported from `index.ts`. A contract with no `networks:` is never built, and `config check` warns about it.

| Key | Meaning |
| --- | --- |
| `type` | What `source` refers to. Required. See [Contract types](#contract-types) |
| `source` | The crate, address, asset or Wasm, in the format `type` expects. Required |
| `args` | Constructor arguments, as a map from `__constructor` parameter name to value |
| `signer` | Account that deploys the contract and runs `after-deploy`. Defaults to the network's `default-account` |
| `after-deploy` | List of contract methods to call, with no arguments, right after deploying |
| `after-deploy-script` | Path to an executable to run after deploying (not yet supported by `build`) |
| `from-network` | Copy the Wasm of a contract deployed on another network and deploy it here (not yet supported by `build`) |
| `networks` | Per-network entries. Only valid at the top level of a contract |

`signer`, `args`, `after-deploy` and `after-deploy-script` only apply when Scaffold deploys the contract, which means `type: workspace` (or `from-network`). Setting them on a contract that only references an existing deployment is an error.

### Contract types

| `type` | `source` format | What Scaffold does |
| --- | --- | --- |
| `workspace` | A crate's package name in `contracts-dir` | Builds, deploys and generates a client |
| `contract` | A deployed contract address (`C…`) | Generates a client bound to that address |
| `registry` | `[namespace/]name@version` from the Stellar Registry | Generates a client bound to the registered deployment |
| `asset` | `CODE:ISSUER`, or `native` | Generates a client for the asset's Stellar Asset Contract |
| `wasm-file` | Path to a local `.wasm` file | Generates a types-only client, with no contract ID |
| `wasm-hash` | A 64-character hex Wasm hash installed on the network | Generates a types-only client |
| `wasm-registry` | `[namespace/]name@version` of a Wasm in the Stellar Registry | Generates a types-only client |

:::caution `build` and `watch` currently support only `workspace` and `contract`. The other types pass `config check` but fail at build time with `uses type: <type>, which build does not support yet`. The same applies to `from-network` and `after-deploy-script`. :::

`registry` and `wasm-registry` sources without an `@version` are accepted with a warning, because the thing they point at can change under you.

A `workspace` contract can only be deployed to a network with deploys allowed. To use the same contract on testnet or mainnet, either set `allow-deploy: true` on that network, or override the entry to point at a deployment you made separately:

```yaml
networks:
  mainnet:
    type: contract
    source: C... # the deployed contract's ID
```

`allow-deploy: true` on a network with the mainnet passphrase is allowed, with a warning.

### Constructor arguments

`args` maps each `__constructor` parameter, by name, to a value. Values are checked against the contract's spec by stellar-cli when it deploys, so write them as you would on the command line: strings, numbers and booleans as YAML scalars, vectors as lists, and structs or maps as maps.

```yaml
args:
  name: ExampleToken
  decimals: 7
  admin: ${account.me}
  initial_supply: 1000000000000000000000000 # 128-bit values keep full precision
  metadata: null # omitted, so an Option parameter becomes None
```

Integers wider than 64 bits are read exactly, so `i128`, `u128` and 256-bit values do not need quoting. `null` leaves the argument out. No shell is involved: values are passed straight to stellar-cli, so `$(…)` is just text.

### `after-deploy`

A list of method names called, in order, with no arguments, signed by the contract's `signer`. They run only when the contract was actually deployed or upgraded on this build. A build that finds the Wasm unchanged skips them.

```yaml
after-deploy: [reset, unpause]
```

### Deploying and redeploying

When a `workspace` contract's Wasm changes, the next build upgrades the existing deployment if the contract supports it, or redeploys it otherwise. Changing only `args` or `after-deploy` does not trigger a redeploy. Run `stellar scaffold clean` to start fresh.

If one contract fails to deploy, the build carries on with the rest, exports what it can, and then exits with an error.

## `extensions`

[Extensions](./extensions.md) run in the order listed, on every network. Each key is an extension name; its value is passed to the extension as its `config`, which is left out when the value is empty. Keys inside the value are passed exactly as written, so use whatever spelling the extension expects.

```yaml
extensions:
  reporter:
    warn_size_kb: 128
  my-ext:
```

## Interpolation

String values can reference other values with `${namespace.name}`.

| Reference | Value | Allowed in |
| --- | --- | --- |
| `${env.NAME}` | Environment variable `NAME` at build time; an error if unset | Network settings and `args` |
| `${env.NAME:-fallback}` | `NAME`, or `fallback` when unset | Network settings and `args` |
| `${account.NAME}` | Address of account `NAME`, which must be listed in the network's `accounts` | `args` |
| `${network.name}`, `${network.rpc-url}`, `${network.passphrase}` | The selected network's resolved values | `args` |

Write `$${` for a literal `${`. The namespaces `contract`, `wasm-hash` and `registry` are reserved for future use and currently rejected.

## Validating your config

```bash
stellar scaffold config check                    # validate the whole file
stellar scaffold config check --network testnet  # also check that testnet is declared
stellar scaffold config show --network testnet   # print the fully resolved config for testnet
```

`config check` reports every problem at once, with the line, a short code such as `unknown-account` or `deploy-not-allowed`, and a suggested fix. It makes no network calls, so it is safe to run in CI. `--strict` also fails on warnings, and `--json` prints diagnostics as JSON.

`config show` applies `extends`, fills in built-in defaults, merges per-network contract overrides and substitutes `${env.…}` and `${network.…}`, so you can see exactly what `build` will use. `${account.…}` references stay as written, since the accounts may not exist yet.

`build`, `watch` and `stellar scaffold doctor` run the same checks.

## Environment variables

| Variable | Effect |
| --- | --- |
| `STELLAR_NETWORK` | Network to build for, same as `--network` |
| `STELLAR_SCAFFOLD_ENV` | Ignored, with a warning. Only read by projects still on `environments.toml` |
| Anything referenced by `${env.…}` | Read at build time |

## Legacy: `environments.toml`

Projects whose `scaffold.yml` says `version: 1` keep their networks, accounts and contracts in `environments.toml`, keyed by environment (`development`, `testing`, `staging`, `production`) and selected with `STELLAR_SCAFFOLD_ENV`. In those projects `scaffold.yml` only holds directories:

```yaml
version: 1

config:
  contracts_dir: contracts
  clients_dir: app-lib/clients
```

```toml
[development]
network = { name = "local", run_locally = true }
accounts = ["me"]

[development.contracts.guess_the_number]
client = true
constructor_args = "--admin me"
after_deploy = "reset"

[production.contracts.guess_the_number]
id = "C..."
```

`build` and `watch` still read this format, and `--network` is ignored for it. To move to version 2:

- Replace `config:` with `project:`, renaming `contracts_dir`/`clients_dir` to `contracts-dir`/`clients-dir`, and set `version: 2`.
- Turn each environment into an entry under `networks:`. `run_locally` becomes `start-container`; an explicit `default = true` account becomes `default-account`.
- Turn each contract into an entry under `contracts:` with `type: workspace` and its crate name as `source`, or `type: contract` and the `id` as `source`, and list the networks it belongs on.
- Rewrite `constructor_args` as an `args` map, `$(stellar keys address x)` as `${account.x}`, and a `STELLAR_ACCOUNT=` prefix as `signer`.
- Rewrite `after_deploy` as a list of method names. Calls that take arguments are not supported by `after-deploy` yet.
- Move `extensions` into the top-level `extensions:` map.
- Replace `STELLAR_SCAFFOLD_ENV` in `.env` with `STELLAR_NETWORK`, then delete `environments.toml`.

Run `stellar scaffold config check` when you are done.
