# Extensions

Scaffold extensions let you tap into the build lifecycle without modifying Scaffold itself. An extension is an ordinary binary on your `PATH` named `stellar-scaffold-<name>`. Scaffold discovers it automatically, asks which hooks it cares about, and calls it at each of those points during a build or watch cycle.

Extensions can do anything: log metrics, run audits, post Slack notifications, write custom artifacts, enforce size budgets, or generate extra files. They receive rich context about what just happened and can write anything they want to stdout, which Scaffold forwards to the user's terminal.

---

## Hook lifecycle

Every build runs through the same ordered sequence of hooks:

```
pre-dev
  └─ pre-compile
       └─ [cargo build per contract]
  └─ post-compile
  └─ pre-deploy   (per contract)
       └─ [upload wasm, deploy/upgrade contract]
  └─ post-deploy  (per contract)
  └─ pre-codegen  (per contract)
       └─ [stellar contract bindings typescript + npm build]
  └─ post-codegen (per contract)
post-dev
```

The `pre-compile` and `post-compile` hooks fire once per build cycle, covering all contracts. The `pre-deploy`, `post-deploy`, `pre-codegen`, and `post-codegen` hooks fire once **per contract**. The `pre-dev` and `post-dev` hooks bookend the entire cycle.

You only need to handle the hooks relevant to your extension. Hooks you do not list in your manifest are never invoked.

---

## Registering an extension

Add your extension to `environments.toml` under the environments where it should run:

```toml
[development]
extensions = ["reporter"]

[staging]
extensions = ["reporter", "audit-tool"]
```

### Per-extension configuration

You can pass arbitrary configuration to an extension via `[<env>.ext.<name>]`:

```toml
[development.ext.reporter]
warn_size_kb = 128
log_file = ".scaffold/reports/dev.log"
```

Scaffold serializes this table and injects it as the `config` field in every hook invocation for that extension. If no config section exists, `config` is absent from the JSON.

---

## How Scaffold calls an extension

1. **Discovery:** On startup, Scaffold runs `stellar-scaffold-<name> manifest` and parses the JSON response to learn which hooks the extension wants.
2. **Invocation:** At each lifecycle point that the extension registered for, Scaffold runs `stellar-scaffold-<name> <hook-name>` and writes a JSON object to its stdin.
3. **Output:** The extension reads stdin, does its work, writes anything it wants to stdout (forwarded to the user's terminal), and exits. A non-zero exit code is logged as an error, but Scaffold continues — remaining extensions registered for the same hook still run and the build is not aborted.

---

## The `manifest` subcommand

Your binary must respond to `manifest` by writing a JSON object to stdout:

```json
{
  "name": "my-extension",
  "version": "1.0.0",
  "hooks": ["post-compile", "post-deploy"]
}
```

Only list hooks your extension actually handles. Listing a hook you do not handle wastes a subprocess invocation on every build. The `name` should match the suffix of your binary (`stellar-scaffold-my-extension` → `"my-extension"`).

---

## The stdin JSON

At each hook invocation, Scaffold writes a flat JSON object to the extension's stdin. The object always includes `config` (your extension's config from `environments.toml`, or `null` if none was provided) plus context fields that depend on which hook is firing.

### Field reference

| Field | pre/post-compile | pre/post-deploy | pre/post-codegen | pre/post-dev |
| --- | --- | --- | --- | --- |
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

### Field descriptions

| Field | Type | Description |
| --- | --- | --- |
| `config` | object \| null | Your extension's config table from `environments.toml`, or `null` |
| `project_root` | string (path) | Absolute path to the Cargo workspace root |
| `env` | string | Active environment: `"development"`, `"testing"`, `"staging"`, or `"production"` |
| `wasm_out_dir` | string (path) | Directory where compiled WASM files are written |
| `source_dirs` | string[] | Contract source directories in topological build order |
| `wasm_paths` | object | Map of `contract_name → wasm_path`; empty at `pre-compile` |
| `network` | object \| null | Resolved RPC URL, network passphrase, and network name |
| `contract_name` | string | Snake-case contract name matching the WASM filename stem |
| `wasm_path` | string (path) | Absolute path to this contract's compiled WASM |
| `wasm_hash` | string | Hex-encoded SHA-256 of the uploaded WASM bytecode |
| `contract_id` | string \| null | Stellar contract address (`C…` strkey); `null` at `pre-deploy` |
| `ts_package_dir` | string (path) | `<project_root>/packages/<name>/` |
| `src_template_path` | string (path) | `<project_root>/src/contracts/<name>.ts` |
| `contracts` | object[] | Per-contract summary array; optional fields are `null` at `pre-dev` |
| `watch_paths` | string[] | Directories being watched; empty in one-shot builds |

### Example: `post-compile` stdin

```json
{
  "config": null,
  "project_root": "/path/to/my-project",
  "env": "development",
  "wasm_out_dir": "/path/to/my-project/target/stellar/local",
  "source_dirs": ["/path/to/my-project/contracts/hello_world"],
  "wasm_paths": {
    "hello_world": "/path/to/my-project/target/stellar/local/hello_world.wasm"
  }
}
```

### Example: `post-deploy` stdin

```json
{
  "config": { "warn_size_kb": 128 },
  "project_root": "/path/to/my-project",
  "env": "development",
  "wasm_out_dir": "/path/to/my-project/target/stellar/local",
  "source_dirs": ["/path/to/my-project/contracts/hello_world"],
  "wasm_paths": {
    "hello_world": "/path/to/my-project/target/stellar/local/hello_world.wasm"
  },
  "network": {
    "rpc_url": "http://localhost:8000/soroban/rpc",
    "network_passphrase": "Standalone Network ; February 2017",
    "network_name": "local"
  },
  "contract_name": "hello_world",
  "wasm_path": "/path/to/my-project/target/stellar/local/hello_world.wasm",
  "wasm_hash": "a1b2c3d4e5f6...",
  "contract_id": "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4"
}
```

---

## Building an extension

### Step 1: Create a binary crate

```sh
cargo new --bin stellar-scaffold-my-extension
cd stellar-scaffold-my-extension
```

### Step 2: Implement the `manifest` subcommand

Your binary must handle `manifest` as its first argument and print JSON to stdout:

```sh
stellar-scaffold-my-extension manifest
```

```json
{
  "name": "my-extension",
  "version": "1.0.0",
  "hooks": ["post-compile", "post-deploy"]
}
```

Only list the hooks you actually handle.

### Step 3: Implement hook handlers

For each hook you listed, handle the corresponding subcommand argument. Read the full stdin JSON, do your work, and print output for the user:

```sh
stellar-scaffold-my-extension post-compile
# (JSON on stdin)
```

### Step 4: Install it on PATH

Scaffold discovers extensions by looking for binaries named `stellar-scaffold-<name>` on your `PATH`. For Rust extensions, install with Cargo:

```sh
cargo install --path .
```

Or copy the compiled binary somewhere on your `PATH`.

### Step 5: Register it in `environments.toml`

```toml
[development]
extensions = ["my-extension"]
```

Run `stellar scaffold build` or `stellar scaffold watch` and your extension will be called at each registered hook.

---

## Language-specific examples

### Rust

Use the [`stellar-scaffold-ext-types`](https://crates.io/crates/stellar-scaffold-ext-types) crate for typed access to the stdin JSON:

```toml
[dependencies]
stellar-scaffold-ext-types = "0.0.1"
serde_json = "1"
```

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
            println!("Compiled {} contracts:", ctx.wasm_paths.len());
            for (name, path) in &ctx.wasm_paths {
                println!("  {name}: {}", path.display());
            }
        }
        _ => {}
    }
}
```

The `ext-types` crate uses `#[serde(flatten)]` so the Rust structs compose naturally while the wire format stays flat. See the [crate README](https://github.com/stellar-scaffold/cli/tree/main/crates/stellar-scaffold-ext-types) for the full type reference.

### TypeScript / Node.js

```ts
import * as readline from "readline";

const args = process.argv.slice(2);

if (args[0] === "manifest") {
  console.log(
    JSON.stringify({
      name: "my-extension",
      version: "1.0.0",
      hooks: ["post-compile"],
    }),
  );
} else if (args[0] === "post-compile") {
  let input = "";
  process.stdin.setEncoding("utf8");
  process.stdin.on("data", (chunk) => (input += chunk));
  process.stdin.on("end", () => {
    const ctx = JSON.parse(input);
    const names = Object.keys(ctx.wasm_paths);
    console.log(`Compiled ${names.length} contracts: ${names.join(", ")}`);
  });
}
```

Install it by putting the script on your `PATH` (via a shebang + `chmod +x`, a compiled bundle with `pkg` or `bun build --compile`, etc.) and naming it `stellar-scaffold-my-extension`.

### Any other language

Extensions are just binaries. Use whatever language you want. The only requirements are:

- The binary is named `stellar-scaffold-<name>` and is on your `PATH`
- Running it with `manifest` prints a JSON manifest to stdout
- Running it with a hook name reads JSON from stdin and exits with code 0 on success

---

## The Scaffold Reporter

`stellar-scaffold-reporter` is the canonical reference implementation. It is the built-in extension that ships with every `stellar scaffold init` project and demonstrates the full hook lifecycle in practice.

It tracks and logs:

- **Compile time** — how long `cargo build` took
- **WASM sizes** — byte size of each contract's `.wasm` output, with delta from the previous build
- **Deploy info** — contract ID, WASM hash, and deploy duration per contract
- **TypeScript package size** — total size of the generated client package
- **Total build cycle duration** — end-to-end time from `pre-dev` to `post-dev`

You can install it standalone with:

```sh
cargo install stellar-scaffold-reporter
```

And register it in `environments.toml`:

```toml
[development]
extensions = ["reporter"]
```

Browse the [source code](https://github.com/stellar-scaffold/cli/tree/main/crates/stellar-scaffold-reporter) and its [README](https://github.com/stellar-scaffold/cli/tree/main/crates/stellar-scaffold-reporter/README.md) to see a complete, real-world extension that handles all eight hooks.
