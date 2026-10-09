# Community Templates

Anyone can publish a frontend template for Scaffold. A community template is a public Git repository that `stellar scaffold init` uses as the starting point of a new project.

```bash
stellar scaffold init my-app --template your-org/your-template
stellar scaffold init my-app --template your-org/your-template#v1.2.0
```

Any `--template` value containing a `/` is treated as a community repo, with an optional `#branch` or `#tag` suffix.

---

## Repo shape

Your repo is used as-is, so it must already be a complete, runnable Scaffold project. At the root it needs:

| Path | Purpose |
| --- | --- |
| `scaffold.yml` | Project config starting with `version: 2` |
| `Cargo.toml` | Rust workspace containing your contracts |
| `.env.example` | _Optional._ Default environment values, copied to `.env` |

Everything else is up to you: where contracts live, and which language, framework, and task runner your frontend uses.

### Project settings

The `project:` section of `scaffold.yml` tells the CLI where things live and which package manager to use:

```yaml
version: 2
project:
  contracts-dir: contracts        # Rust contract crates
  clients-dir: app-lib/clients    # Generated TypeScript clients
  package-manager: pnpm           # npm, pnpm, yarn, bun, or deno; pin with pnpm@9.6.0
```

All three are optional. Directories default to the values above and the package manager defaults to npm. `init` uses your template's `package-manager` unless the user passes `--package-manager`. See [Configuration](/docs/configuration) for every `scaffold.yml` key.

### Generated clients

During `init` and `stellar scaffold build --build-clients`, the CLI writes one TypeScript package per contract into `clients-dir`, plus an `index.ts` exporting a ready-to-use client for each. Import clients from that file wherever your frontend lives.

`index.ts` only imports the packages beside it, so it works from any `clients-dir`. Network settings are written in at build time, so rebuild to target a different network.

By default, plain HTTP RPC URLs are allowed only on the local network. To allow them elsewhere, opt in on that network:

```yaml
networks:
  devnet:
    rpc-url: http://devnet.internal:8000/rpc
    network-passphrase: Devnet ; 2026
    allow-http: true
```

If your frontend doesn't use a contract's client, for example a library contract other contracts depend on, or a frontend that isn't JavaScript, turn it off. The contract still builds and deploys:

```yaml
contracts:
  token:
    type: workspace
    source: token
    client: false
    networks:
      local:
```

`client` applies to every network, so it can't be set under a contract's `networks:` entries.

Building clients needs Node and your package manager installed. A template whose contracts all set `client: false` needs neither.

---

## Maintaining a template

- **Start from an official template.** `stellar scaffold init my-template --template react`, then swap in your own frontend.
- **Pin the CLI version.** If your template has a root `package.json`, set `engines.stellar-scaffold` (e.g. `">=0.0.27 <0.1.0"`). Users with an incompatible CLI get a clear error instead of a broken project.
- **Tag releases.** Let users pin `your-org/your-template#v1.2.0` so your `main` branch can move without breaking them.
- **Keep it public.** `init` downloads the repo without credentials, so private repos won't work.
- **Track upstream.** Watch [`stellar-scaffold/ui`](https://github.com/stellar-scaffold/ui) and the [CLI changelog](https://github.com/stellar-scaffold/cli/blob/main/crates/stellar-scaffold-cli/CHANGELOG.md) for changes to `scaffold.yml` or generated client output.
- **Test in CI.** Run `stellar scaffold init tmp --template your-org/your-template#<branch> -y` and then your build and test scripts against each CLI release you support.
- **Document setup.** Note any required env vars, Docker usage, or third-party extensions in your README.
