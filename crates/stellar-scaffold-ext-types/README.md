# stellar-scaffold-ext-types

Shared types for [Scaffold Stellar](https://scaffoldstellar.com) extension hooks.

Extensions are binaries on your `PATH` named `stellar-scaffold-<name>`. Scaffold discovers them, calls `manifest` to learn which hooks they want, then invokes them at each registered lifecycle point by writing a JSON object to stdin.

This crate gives Rust extension authors typed, zero-boilerplate access to that JSON. If you are writing an extension in another language you do not need this crate — just parse the raw JSON directly.

---

## Installation

Add the crate to your extension's `Cargo.toml`:

```toml
[dependencies]
stellar-scaffold-ext-types = "0.0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

---

## Types

### `HookName`

An exhaustive enum of every lifecycle hook Scaffold can fire, serialized as kebab-case strings:

| Variant | String |
|---|---|
| `HookName::PreCompile` | `"pre-compile"` |
| `HookName::PostCompile` | `"post-compile"` |
| `HookName::PreDeploy` | `"pre-deploy"` |
| `HookName::PostDeploy` | `"post-deploy"` |
| `HookName::PreCodegen` | `"pre-codegen"` |
| `HookName::PostCodegen` | `"post-codegen"` |
| `HookName::PreDev` | `"pre-dev"` |
| `HookName::PostDev` | `"post-dev"` |

Use `HookName::as_str()` to get the string form needed in a manifest's `hooks` array.

### `ExtensionManifest`

The JSON your binary must write to stdout when called with the `manifest` subcommand:

```rust
ExtensionManifest {
    name: String,    // e.g. "my-extension"
    version: String, // SemVer, e.g. "1.0.0"
    hooks: Vec<String>, // only the hooks you handle
}
```

### Context structs

Each hook receives one context type on stdin. The types form a strict information superset going down the build chain:

```
CompileContext  ──  pre-compile, post-compile
    ⊂
DeployContext   ──  pre-deploy, post-deploy
    ⊂
CodegenContext  ──  pre-codegen, post-codegen

ProjectContext  ──  pre-dev, post-dev  (aggregates all contracts)
```

`DeployContext` composes `CompileContext` via `#[serde(flatten)]`. The wire format is a flat JSON object, but in Rust you access compile-stage fields through `ctx.compile.*`, e.g. `ctx.compile.project_root`.

`CodegenContext` composes `DeployContext` the same way: compile fields are at `ctx.deploy.compile.*`.

#### `config` field

Every context type carries the extension's own configuration from the `extensions:` section of `scaffold.yml` (or from `environments.toml` in projects that still use it). Where it lives depends on the hook:

| Hook group | Access path |
|---|---|
| pre/post-compile | `ctx.config` |
| pre/post-deploy | `ctx.compile.config` |
| pre/post-codegen | `ctx.deploy.compile.config` |
| pre/post-dev | `ctx.config` |

The field is typed `Option<serde_json::Value>`. When no config is provided it is `None` and is absent from the JSON entirely (`#[serde(skip_serializing_if = "Option::is_none")]`). Scaffold always injects the real value from the project's config when it is present.

---

## Wire format

All hooks receive a **flat** JSON object on stdin regardless of how the Rust structs compose. Fields present at each hook:

| Field | pre/post-compile | pre/post-deploy | pre/post-codegen | pre/post-dev |
|---|---|---|---|---|
| `config` | ✓ | ✓ | ✓ | ✓ |
| `project_root` | ✓ | ✓ | ✓ | ✓ |
| `env` | ✓ | ✓ | ✓ | ✓ |
| `wasm_out_dir` | ✓ | ✓ | ✓ | ✓ |
| `source_dirs` | ✓ | ✓ | ✓ | ✓ |
| `wasm_paths` | ✓ (empty at pre) | ✓ | ✓ | — |
| `network` | — | ✓ | ✓ | ✓ (if `--build-clients`) |
| `contract_name` | — | ✓ | ✓ | — |
| `wasm_path` | — | ✓ | ✓ | — |
| `wasm_hash` | — | ✓ | ✓ | — |
| `contract_id` | — | ✓ (`null` at pre) | ✓ | — |
| `ts_package_dir` | — | — | ✓ | — |
| `src_template_path` | — | — | ✓ | — |
| `contracts` | — | — | — | ✓ |
| `watch_paths` | — | — | — | ✓ |

### `network` object

```json
{
  "rpc_url": "http://localhost:8000/soroban/rpc",
  "network_passphrase": "Standalone Network ; February 2017",
  "network_name": "local"
}
```

`network_name` is `null` when the network was configured with explicit `rpc_url`/`network_passphrase` rather than a named preset.

### `wasm_paths` object

```json
{
  "hello_world": "/path/to/project/target/stellar/local/hello_world.wasm",
  "another_contract": "/path/to/project/target/stellar/local/another_contract.wasm"
}
```

Empty object `{}` at `pre-compile`.

### `contracts` array (pre-dev / post-dev only)

Array of per-contract objects. All optional fields are `null` at `pre-dev` and populated at `post-dev` for contracts that were successfully processed:

```json
[
  {
    "name": "hello_world",
    "source_dir": "/path/to/project/contracts/hello_world",
    "wasm_path": "/path/to/project/target/stellar/local/hello_world.wasm",
    "wasm_hash": "abc123...",
    "contract_id": "CAAAAAAA...",
    "ts_package_dir": "/path/to/project/packages/hello_world",
    "src_template_path": "/path/to/project/src/contracts/hello_world.ts"
  }
]
```

---

## Minimal example

```rust
use std::io::Read;
use stellar_scaffold_ext_types::{ExtensionManifest, HookName, CompileContext};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("manifest") => {
            let manifest = ExtensionManifest {
                name: "my-extension".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                hooks: vec![HookName::PostCompile.as_str().to_string()],
            };
            println!("{}", serde_json::to_string(&manifest).unwrap());
        }
        Some("post-compile") => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).unwrap();
            let ctx: CompileContext = serde_json::from_str(&buf).unwrap();

            println!("Build finished. WASM files:");
            for (name, path) in &ctx.wasm_paths {
                println!("  {name}: {}", path.display());
            }
        }
        _ => {}
    }
}
```

For a complete, production-quality example see the [`stellar-scaffold-reporter`](../stellar-scaffold-reporter) crate. It is the built-in Scaffold extension that tracks compile times, WASM sizes, deploy info, and total build cycle duration.

---

## Non-Rust extensions

Extensions are ordinary binaries. You can write them in any language. The JSON Scaffold sends on stdin is a plain flat object — just read stdin and parse it with your language's JSON library. You do not need this crate.

See the [Scaffold Extensions documentation](https://scaffoldstellar.com/docs/extensions) for language-agnostic examples and the full field reference.
