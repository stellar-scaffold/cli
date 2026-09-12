After a bunch of research, here's my proposed schema and reasoning:

## 1. Design principles

1. Switch the axis from "environments" to networks, but allow custom ones. Network keys matching the built-in ones get their defaults, custom keys can either extend and override or have their own explicit config.
2. Contracts first, networks second. A client is configured, then overrides are provided explicitly per network.
3. Contract source defines its config shape. A path points to within the repo vs a contract id or classic asset or Wasm hash or Registry name.
4. Typed and validated where possible. Args are a map checked against a constructor. After-deploy is either a list of method names (no args allowed), or a path to an executable script.
5. Intent rather than result. This config declares what should happen, the CLI won't rewrite it. And `.env` helps select the network.

## 2. Proposed Schema

```yml
version: 2

project:
  contracts-dir: contracts       # showing default, can be omitted
  clients-dir: app-lib/clients    # showing default, omitting means no client generation
  optimize: true                       # `stellar contract build --optimize`, cli#329

extensions:                            # map order = run order; value = that extension's config
  reporter:
    warn-size-kb: 128
  debugger: {}

networks:
  local:                               # built-in: rpc/passphrase from stellar-cli
    accounts: [me, admin]
    default-account: admin             # required when accounts has > 1 entry
    # start-container: true            # default, derived from the standalone passphrase
    # allow-deploy: true               # default for local and custom names

  testnet:
    accounts: [testnet-deployer]
    # allow-deploy: false              # default for testnet/mainnet/futurenet/pubnet

  mainnet:
    rpc-url: https://mainnet.example.com
    rpc-headers:
      Authorization: ${env.MAINNET_RPC_TOKEN}
    accounts: [deployer]               # must already exist in the keystore; never generated

  preview:                             # custom network: same chain as testnet, own contract set.
    extends: testnet                   # this is what "environments" used to express
    accounts: [ci-deployer]
    allow-deploy: true                 # inherited false; must opt in explicitly

contracts:                    # not necessarily client config anymore
  my-token:                            # client name → npm package `my-token`, export `myToken`
    source: ./contracts/fungible_token_interface_example   # a path = a crate we build and deploy
    networks:
      local:
        args:
          owner: ${account.admin}
          initial_supply: "1000000000000000000000000"   # i128 → quoted decimal string
        after-deploy: [reset]
      preview:
        signer: ci-deployer
        args: { owner: ${account.ci-deployer}, initial_supply: "1000000" }
        after-deploy-script: ./scripts/seed-preview.ts   # relative to scaffold.yml
      testnet:
        source: registry:circle/usdc                      # per-network override of source

  dex:                                 # third-party, consumed only
    source: CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC   # a strkey = deployed contract
    networks:
      testnet: {}                      # exists here, inherits everything
      local:
        from-network: testnet          # fetch Wasm from testnet, deploy locally (cli#346, no spooning)

  usdc:                                # classic asset; SAC id derived, wrapper deployed idempotently
    networks:
      testnet:
        source: USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5   # CODE:ISSUER = classic asset

  guess-the-number:
    source: ./contracts/guess_the_number
    signer: me
    args: { admin: ${account.me} }
    after-deploy: [reset]
    networks:
      local: {}
      mainnet:
        source: CABC...                # referenced there; deploy keys forbidden with a non-path source
```


## 3 Schema Details

### 3.1 `version`
Integer schema epoch. Bump only for breaking parse changes. We can map schema-version to minimum Scaffold version so errors read "schema 3 requires Scaffold ≥ 1.4.0". Unknown keys are rejected everywhere, so a new field also needs a newer CLI.

### 3.2 `project`
| Key | Default |
|---|---|
| `contracts-dir` | `contracts` |
| `clients-dir` | `app-lib/clients` |
| `optimize` | `false`; passes `--optimize` to `stellar contract build` (cli#329) |

### 3.3 `networks.<name>`
| Key | Type | Default | Notes |
|---|---|---|---|
| `extends` | string | — | single inheritance; chain depth ≤ 3; cycles rejected |
| `rpc-url`, `network-passphrase` | string | from stellar-cli for built-in names | required for custom names unless inherited |
| `rpc-headers` | map string→string | `{}` | a map, not a list of pairs |
| `accounts` | list of strings | `[]` | generated + funded via friendbot if absent and the chain has one; verified-only on mainnet |
| `default-account` | string | sole entry of `accounts` | **required when `accounts` has > 1 entry**. Explicit key, because list position must not decide who signs mainnet transactions |
| `start-container` | bool | true for local/standalone passphrases | renamed from `run-locally`; derived from passphrase so custom local networks work; `true` on a public passphrase is an error |
| `allow-deploy` | bool | true for local and custom names, false for public built-ins | policy; inherited through `extends` |

The possibility of `extends` cleans up the config, but then we need merge rules. So resolve the parent fully (including name-derived defaults), then apply the child. Scalars override, maps merge, lists replace. A child that replaces `accounts` can orphan an inherited `default-account`; validation catches it. We could have `stellar scaffold config show --network <n>` print the resolved result.

### 3.4 `contract-clients.<name>` and `.networks.<network>`
Keys directly under `<name>` are defaults for every network; keys under `networks.<n>` override by the same rules. An empty `networks.<n>: {}` means "exists here, inherits everything". A client with no entry for the selected network is a **build error**.

| Key | Type | Notes |
|---|---|---|
| `source` | string | required at some level; see 3.5 |
| `from-network` | string | fetch Wasm from another network, deploy here; only with a contract id, `registry:` or Wasm hash source |
| `signer` | string | account alias; replaces `STELLAR_ACCOUNT=`; defaults to network's `default-account` |
| `args` | map | constructor args by **contract parameter name** (snake_case, contract-owned) |
| `after-deploy` | list of strings | no-argument method names, in order |
| `after-deploy-script` | path | any executable relative to `scaffold.yml`; receives JSON context on stdin (same shape as extension hooks) |

`signer`, `args`, `after-deploy`, `after-deploy-script` are **schema errors** with any source that is not a crate path.

### 3.5 Source grammar
One string; its shape decides the kind. Paths are paths, on-chain things are their canonical string. Only the registry needs a prefix, because a bare `name@version` looks like nothing else on this table. 

| Shape | Kind | Binds an id | Deployable |
|---|---|---|---|
| `./<dir>` (a directory) | crate in the cargo workspace; built here | yes | yes |
| `./<file>.wasm` | Wasm file; types-only client | no | no |
| `C…` 56-char strkey | deployed contract | yes | no |
| `CODE:GISSUER…` or `native` | classic asset; SAC id derived | derived | wrapper only, idempotent |
| 64 hex chars | Wasm hash on this network; types-only | no | no |
| `registry:[namespace/]name[@version]` | Stellar Registry contract | yes | no |

**If we want better error UX, scheme prefixes would help.** Think of Deno's import module prefixes. With bare shapes, `CABC…` with one wrong character must produce "looks like a contract id but the checksum failed", `./contracts/gues_the_number` must produce "no crate at that path; did you mean `guess_the_number`?", and `USDC:GXYZ` must produce "looks like an asset but the issuer is not a valid G-address". That is a check-type-then-validate step per kind instead of one "unknown scheme" error.

### 3.6 Interpolation
`${namespace.path}` in any string value, resolved before validation. Parse generically; reject unknown namespaces listing the known ones, so the grammar is reserved now and later namespaces are additive.

| Namespace | Resolves to | v2 |
|---|---|---|
| `${account.<alias>}` | `G…` of a configured account on this network | implement |
| `${env.VAR}`, `${env.VAR:-default}` | environment variable | implement |
| `${network.name}`, `${network.rpc-url}`, `${network.passphrase}` | resolved network | implement |
| `${contract.<name>}`, `${wasm-hash.<name>}`, `${registry.<name>}` | sibling client id / built hash / registry lookup | reserve for now |

### 3.7 Capability derivation
What does `build` command actually do for a given listed contract? There's **5** possibilities and nothing keys off a network's name anymore:
```
build    ⟸ source is a crate path
deploy   ⟸ source is a crate path && network.allow-deploy
         || source is an asset && SAC wrapper absent
seed     ⟸ deploy permitted
upgrade  ⟸ never from this file
codegen  ⟸ if project.clients-dir is set
```

## 4. Selecting network and accounts
- `accounts` is a list that we'll **ensure** exists. Networks with Friendbot will generate + fund. Mainnet will verify only and error if absnet.
- Signer is specified per network per contract, or falls back to network default.
- `--network <name>` on `build`, `watch`, `check`, `config show`; then `STELLAR_SCAFFOLD_NETWORK`; then `local`. `STELLAR_SCAFFOLD_ENV` is aliased with a warning during 0.0.x and removed at 1.0.
- Container start + health check only when the resolved network has `start-container: true`.

## 5. Validation and migration
- We're already parsing config files and failing on some errors. The `scaffold doctor` command is aggregating most of that, but also handles tooling and environment stuff. We should probably surface a new command to check config only (which `doctor` can consume).
- Some validation rules: version in range, networks per client exist, interpolation for namespaced values, spec checking args, etc.
- We should make migration as painless as possible with a migration script, which we can maintain over time.

## 6. Open Decisions?
1. We good with YAML? I do think that editing yml files programmatically will be harder to maintain, but we're not doing that, _yet_. Which leads me to...
2. Should we create a `scaffold.lock` file too? Now that we allow custom network names, the global alias store keyed by passphrase could collide. So we either store deployed ids in a lockfile or we limit scaffold to deploy to local container networks.
3. Do we want to be explicit and source prefixes? See the note in 3.5.
4. I kinda don't like `default-account` cause it seems excessive. Is it too auto-magic use the first element of the accounts list?

