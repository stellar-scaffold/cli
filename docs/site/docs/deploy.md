# Deployment

## Smart contract

When you are ready for testnet/mainnet, we recommend to deploy your contract using `stellar registry`. The registry has two namespaces:

- **Verified registry** (default) - Requires manager approval for initial publishes and deploys
- **Unverified registry** - Open for anyone to publish (use `unverified/` prefix)

Some commands to get you started:

```bash
#  Note --source-account argument is omitted for clarity

# First publish your contract to the unverified registry
stellar registry publish \
  --wasm target/stellar/local/my_contract.wasm \
  --wasm-name unverified/my-contract \
  --binver "1.0.0"

# Then deploy an instance with constructor parameters
stellar registry deploy \
  --contract-name unverified/my-contract-instance \
  --wasm-name unverified/my-contract \
  -- \
  --param1 value1

# Can access the help docs with --help
stellar registry deploy \
  --contract-name unverified/my-contract-instance \
  --wasm-name unverified/my-contract \
  -- \
  --help

# Install the deployed contract locally
stellar registry create-alias unverified/my-contract-instance
```

**Note:** Names are normalized - underscores become hyphens and uppercase becomes lowercase.

Additionally, you might want to have a look into registering your contract with [Stellar.Expert](https://stellar.expert/explorer/public/contract/validation).

We provide a template GitHub action which you have to adjust based on your needs. This action compiles your contract(s), create some signed attestations and registers the specific Wasm to Stellar.Expert. You then need to download and upload that specific Wasm using the aforementioned `stellar registry` commands.

## dApp

Once you are ready to deploy your dApp, you can run:

```bash
npm run build
```

This will bundle your application in the `/dist` folder. Then you have mainly two approaches to serve its content:

1. A centralized service provider like GitHub Pages, Vercel, Netlify, etc.
2. A decentralized solution like IPFS.

As we are developing a blockchain application and decentralization is one of the core principle of what we do, we recommend to deploy on your application following a decentralized approach.

If for some reason you still prefer a centralized solution, you can have a look at this extensive guide from [Vite](https://vite.dev/guide/static-deploy).

The rest of this guide will focus on a decentralized deployment. We provide a GitHub action workflow which builds the application, creates a signed attestation and deploys the files to IPFS:

1. Push on `dapp_production`,
2. The IPFS workflow runs and produces an IPFS address (CID),

The CID can be used with any IPFS gateway. This might not seem very convenient for your users, luckily in the ecosystem we have a gateway with which it is very easy to integrate:

https://xlm.sh/

Thanks to that provider, you can have a website under `your-dapp.xlm.sh`.

IPFS is not magical in the sense that we have some setup to do. You need an entry point to upload your files to if you don't yourself want to run an IPFS node. Our GitHub action integrates with Storacha and following are some instructions.

### Storacha setup

Follow Storacha's [documentation](https://docs.storacha.network) to create a Space and obtain its Proof (a UCAN proof string). Following is a quick start.

If you do not have an account, you will be prompted to create one and select a plan. The free plan is more than enough for what we want to do.

Install the CLI:

```bash
npm install @storacha/cli
```

Create an account or log in:

```bash
storacha login
```

Create a space:

```bash
storacha space create scaffold
```

This will generate a space which is identifiable by a DID, something like `did:key:z6Mk...`.

The next step is to create a key, this is `STORACHA_PRINCIPAL`:

```bash
storacha key create --json
```

The key itself has its own DID. Use that DID to create a delegation proof:

```bash
export AUDIENCE=did:key:z6Mk...
storacha delegation create $AUDIENCE -c space/blob/add -c space/index/add -c filecoin/offer -c upload/add --base64
```

This is your `STORACHA_PROOF`.
