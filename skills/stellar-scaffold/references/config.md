# Configuration reference

A Scaffold project has three config surfaces:

| File | Holds | Committed |
|---|---|---|
| `scaffold.yml` | Networks, accounts, contracts, extensions, project directories | Yes |
| `.env` | `STELLAR_NETWORK`, `PUBLIC_STELLAR_*` for the frontend, secrets referenced by `${env.…}` | No (copy from `.env.example`) |
| `Cargo.toml` | Rust workspace (`members = ["contracts/*"]`), per-crate `[package.metadata.stellar]` contract metadata | Yes |

## `scaffold.yml` (schema version 2)

```yaml
version: 2 # required, must be 2

project:
  contracts-dir: contracts # default
  clients-dir: app-lib/clients # default

networks:
  local:
    accounts: [me, alice]
    default-account: me
  testnet:
    accounts: [testnet-user]
    allow-deploy: true
  preview: # a second setup on testnet, with its own accounts
    extends: testnet
    accounts: [preview-user]
  mainnet:
    rpc-url: https://your-rpc-provider.example
    rpc-headers:
      Authorization: "Bearer ${env.MAINNET_RPC_TOKEN}"
    accounts: [mainnet-user]

contracts:
  my-token:
    type: workspace
    source: fungible-token
    signer: alice
    args:
      owner: ${account.alice}
      initial_supply: 1000000000000000000000000
    after-deploy: [unpause]
    networks:
      local:
      testnet:
        signer: testnet-user
        args:
          owner: ${account.testnet-user}
      mainnet:
        type: contract
        source: CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC

extensions:
  reporter:
    warn_size_kb: 128
```

The parser is strict: unknown keys, duplicate keys and wrong types are errors. Keys are kebab-case. Names of networks and accounts use letters, digits, `-` and `_`, starting with a letter or digit. A bare `name:` entry is the same as `name: {}`.

### `networks`

| Key | Default | Meaning |
|---|---|---|
| `rpc-url` | stellar-cli's URL for `local`, `testnet`, `futurenet` | Required for `mainnet` and custom names unless inherited |
| `network-passphrase` | stellar-cli's passphrase for built-in names | Required for custom names unless inherited |
| `rpc-headers` | none | Map of headers sent with every RPC request |
| `accounts` | none | Aliases Scaffold creates (`stellar keys generate`) and funds where it can |
| `default-account` | first of `accounts` | Signs deploys when a contract has no `signer`. Must be in `accounts` |
| `start-container` | `true` only for the standalone passphrase | Start a `stellar/quickstart` container. `false` if a chain already runs at `rpc-url` |
| `allow-deploy` | `false` for testnet, futurenet, mainnet passphrases; else `true` | Whether `workspace` contracts may deploy here. `true` on mainnet is a warning |
| `extends` | none | Inherit another declared network's settings |

Defaults come from the **passphrase**, never the name: a custom network on testnet's passphrase behaves like testnet.

`extends` merge rules (also used for contract per-network overrides): scalars override, maps merge, lists replace. An explicit `allow-deploy` is inherited; an unset one is recomputed from the child's passphrase. Chains are at most three hops, with no cycles.

Network fields may use `${env.…}` only.

### `contracts`

The key is the **client name**: the package directory `<clients-dir>/<name>/` and, camelCased, the export from `index.ts` (`my-token` → `myToken`). It is independent of the crate name. It is also the contract alias Scaffold saves, so `stellar contract invoke --id my-token --network local -- …` works after a deploy.

| Key | Meaning |
|---|---|
| `type` | Required. What `source` is. Must be paired with `source` |
| `source` | Required. Format depends on `type` |
| `args` | Constructor arguments by `__constructor` parameter name |
| `signer` | Account that deploys and runs `after-deploy`. Defaults to the network's `default-account` |
| `after-deploy` | List of no-argument methods called, in order, right after a deploy or upgrade |
| `after-deploy-script` | Path to an executable run after deploy (**not yet supported by `build`**) |
| `from-network` | Copy another network's deployed Wasm and deploy it here (**not yet supported by `build`**) |
| `networks` | Per-network entries. **Required**: the contract exists only on networks listed here |

Keys at the contract's top level are defaults for every listed network. Per-network entries override them. Changing `type` in an override requires `source` too.

`signer`, `args`, `after-deploy` and `after-deploy-script` only apply to contracts Scaffold deploys (`type: workspace`). Setting them on a reference type is an error.

#### Contract types

| `type` | `source` | `build` support |
|---|---|---|
| `workspace` | Crate package name in `contracts-dir` (`-` and `_` are interchangeable) | Builds, deploys, generates client |
| `contract` | Deployed address, `C…` | Generates a client bound to it |
| `registry` | `[namespace/]name@version` | Not yet |
| `asset` | `CODE:ISSUER` or `native` | Not yet |
| `wasm-file` | Path to a `.wasm` (types-only client) | Not yet |
| `wasm-hash` | 64 hex chars (types-only client) | Not yet |
| `wasm-registry` | `[namespace/]name@version` (types-only client) | Not yet |

Unpinned `registry`/`wasm-registry` sources (no `@version`) are a warning. A `workspace` contract on a network with `allow-deploy: false` is an error: allow deploys there, or override that network with `type: contract`.

#### Constructor `args`

Written as YAML values and checked by stellar-cli against the contract's spec at deploy time:

```yaml
args:
  name: ExampleToken # String / Symbol
  decimals: 7 # u32
  admin: ${account.me} # Address (the account alias resolves to its G… address)
  initial_supply: 1000000000000000000000000 # i128/u128/u256 read exactly, no quotes needed
  recipients: ["${account.me}", "${account.alice}"] # Vec; quote ${…} inside [ ] or { }
  config: { fee_bps: 30, paused: false } # struct / map
  metadata: null # left out, so an Option parameter is None
```

No shell is involved: `$(stellar keys address me)` is passed as literal text. Use `${account.me}`.

### Interpolation

| Reference | Value | Allowed in |
|---|---|---|
| `${env.NAME}` | Environment variable; error if unset | Network fields, `args` |
| `${env.NAME:-default}` | Variable, or `default` if unset | Network fields, `args` |
| `${account.NAME}` | That account's address; must be in the network's `accounts` | `args` |
| `${network.name}`, `${network.rpc-url}`, `${network.passphrase}` | The selected network's resolved values | `args` |

`$${` is a literal `${`. `contract`, `wasm-hash` and `registry` namespaces are reserved and rejected today.

### `extensions`

Map of extension name to its config, run in order on every network. The value is passed verbatim as the extension's `config` JSON (keys keep their spelling, e.g. the reporter reads `warn_size_kb`). Extensions are binaries named `stellar-scaffold-<name>` on `PATH`.

## `.env`

```bash
STELLAR_NETWORK=local                       # network the CLI builds and deploys for
XDG_CONFIG_HOME=".config"                   # keeps keys and aliases inside the project

PUBLIC_STELLAR_NETWORK="LOCAL"              # LOCAL | TESTNET | FUTURENET | PUBLIC (mainnet)
PUBLIC_STELLAR_NETWORK_PASSPHRASE="Standalone Network ; February 2017"
PUBLIC_STELLAR_RPC_URL="http://localhost:8000/rpc"
PUBLIC_STELLAR_HORIZON_URL="http://localhost:8000"
```

`PUBLIC_*` variables are exposed to the frontend by Vite. Keep them on the same network as `STELLAR_NETWORK`. `.env.example` in the project lists the testnet and mainnet values.

The `stellar` CLI loads `.env` from the current directory before running a plugin, so `stellar scaffold …` (and `npm run dev`) see `STELLAR_NETWORK`. Running the `stellar-scaffold` binary directly does not load `.env`.

## `Cargo.toml` contract metadata

```toml
[package.metadata.stellar]
cargo_inherit = true            # copy name, authors, homepage → home_domain, repository → source_repo, version → binver (SEP-47)
name = "my-awesome-contract"    # override an inherited value
```

`build` writes this metadata into each contract's Wasm.

## Legacy: `environments.toml`

Projects whose `scaffold.yml` says `version: 1` (only a `config:` section with `contracts_dir`/`clients_dir`) keep networks, accounts and contracts in `environments.toml`, per environment (`development`, `testing`, `staging`, `production`), selected by `STELLAR_SCAFFOLD_ENV`. `build` still reads them, and ignores `--network`.

To convert:

- `version: 2`; `config:` → `project:` with `contracts-dir`/`clients-dir`.
- Each `[<env>.network]` → a `networks:` entry. `run_locally` → `start-container`. An account with `default = true` → `default-account`.
- Each `[<env>.contracts.<name>]` → one `contracts.<name>` entry with `type: workspace` + `source: <crate>` (or `type: contract` + `source: <id>` where it had `id`), listing each network it belongs on.
- `constructor_args = "--admin me"` → `args: { admin: ${account.me} }`. `$(stellar keys address x)` → `${account.x}`. `STELLAR_ACCOUNT=x` prefix → `signer: x`.
- `after_deploy` → `after-deploy: [method, …]`, no-argument calls only.
- `client = false` has no equivalent. Every listed contract gets a client.
- `extensions = [...]` and `[<env>.ext.<name>]` → the top-level `extensions:` map.
- `.env`: `STELLAR_SCAFFOLD_ENV=…` → `STELLAR_NETWORK=…`. Delete `environments.toml`.

Run `stellar scaffold config check` until clean.
