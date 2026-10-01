# Scaffold Reporter

An extension for [Scaffold Stellar](https://scaffoldstellar.org) that logs useful build metrics to your console during every compile, deploy, and watch cycle.

## 📐 What Gets Reported?

- **Compile time:** How long `cargo build` took for all contracts in the workspace.
- **WASM size:** Size in KB for each compiled contract, with a delta from the previous build (e.g. `▲ 240B` or `▼ 80B`). Every byte matters. Storage on Stellar costs fees, and there is a hard cap near 128KB.
- **WASM hash:** The SHA-256 of the uploaded bytecode. Two identical compilations produce the same hash; the network deduplicates uploads automatically.
- **Deploy vs. upgrade:** Fresh deploys create a new contract address. Upgrades swap the code at the existing address while preserving all stored data. The reporter tells you which one happened.
- **Deploy duration:** How long the upload + deploy/upgrade step took, per contract.
- **TypeScript package size:** Size of the generated client package bundled with your frontend.
- **Total build time:** End-to-end latency from file save to regenerated frontend client. Your core development feedback loop!

## 📦 Installation

If you started your project with `stellar scaffold init`, the reporter is already included.

To add it to an existing project, install the binary with [Cargo](https://doc.rust-lang.org/cargo/commands/cargo-install.html):

```sh
cargo install stellar-scaffold-reporter
```

Then register it in your `scaffold.yml`:

```yaml
extensions:
  reporter:
```

## 🏃 Running

That's it. The reporter runs automatically whenever you use `stellar scaffold build` or `stellar scaffold watch`. Output is written to your console alongside Scaffold's own output.

## ⚙️ Configuration

No configuration is required. The reporter works out of the box with sensible defaults.

More configuration options will be documented here as they are introduced.
