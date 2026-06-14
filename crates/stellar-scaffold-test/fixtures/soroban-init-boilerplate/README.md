# Stellar Project

## Project Structure

This repository uses the recommended structure for a Soroban project:

```text
.
├── contracts
│   └── hello_world
│       ├── src
│       │   ├── lib.rs
│       │   └── test.rs
│       └── Cargo.toml
├── Cargo.toml
└── README.md
```

- New Soroban contracts can be put in `contracts`, each in their own directory. There is already a `hello_world` contract in there to get you started.
- If you initialized this project with any other example contracts via `--with-example`, those contracts will be in the `contracts` directory as well.
- Contracts should have their own `Cargo.toml` files that rely on the top-level `Cargo.toml` workspace for their dependencies.
- Frontend libraries can be added to the top-level directory as well. If you initialized this project with a frontend template via `--frontend-template` you will have those files already included.

---

<!-- The following is the Frontend Template's README.md -->

# Soroban Frontend in Astro

A Frontend Template suitable for use with `soroban contract init --frontend-template`, powered by [Astro](https://astro.build/).

# Getting Started

- `cp .env.example .env`
- `npm install`
- `npm run dev`

# How it works

If you look in [package.json](./package.json), you'll see that the `start` & `dev` scripts first run the [`initialize.js`](./initialize.js) script. This script loops over all contracts in `contracts/*` and, for each:

1. Deploys to a local network (_needs to be running with `docker run` or `soroban network start`_)
2. Saves contract IDs to `.soroban/contract-ids`
3. Generates TS bindings for each into the `bindings` folder, which is set up as an [npm workspace](https://docs.npmjs.com/cli/v10/configuring-npm/package-json#workspaces)
4. Create a file in `core/clients` that imports the contract client and initializes it for the `local` network.

You're now ready to import these initialized contract clients in your [Astro templates](https://docs.astro.build/en/core-concepts/astro-syntax/) or your [React, Svelte, Vue, Alpine, Lit, and whatever else JS files](https://docs.astro.build/en/core-concepts/framework-components/#official-ui-framework-integrations). You can see an example of this in [index.astro](./src/pages/index.astro).
