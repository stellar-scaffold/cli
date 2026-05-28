---
sidebar_label: Getting Started
---

# Getting Started with Scaffold Stellar

This section will guide you through the development workflow for using Scaffold Stellar to build and deploy a Guess the Number game with a simple smart contract and an integrated frontend application.

:::tip

If you just want to get up and running quickly, check out the [Quick Start](../quick-start.mdx) guide.

:::

We'll cover:

1. [Setting up a development environment](#%EF%B8%8F-setup-your-development-environment)
2. [Initializing a new project](#%EF%B8%8F-initialize-your-project)
3. [Running the application](#-open-the-app)
4. [Exploring the scaffolded project structure](#%EF%B8%8F-exploring-the-project-structure)
5. [Understanding code in an example contract](#-understand-the-contract-code)
6. [Understanding how the application talks to the contract](#-understand-the-application-code)

## 🛠️ Setup Your Development Environment

First, follow the [Setup Instructions](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup) here to install the necessary tools for Stellar contract development, specifically these sections:

- Install Rust, Cargo (for managing Rust projects), and the compilation target
- Configure your editor for Rust development
- Install the Stellar CLI

To work with Scaffold Stellar, we'll need a few more things.

### Node

Go to [the Node.js download page](https://nodejs.org/en/download) and follow the instructions to the the LTS version on your operating system. You can also use a version manager like `nvm` or install using Homebrew if you prefer. This should also install `npm` as well.

```sh
brew install node@22

# Verify installation
node -v # should print "v22.20.0" or higher
npm -v # should print "10.9.3" or higher
```

### Docker

We'll run a local Stellar network inside a Docker container, so head to the [Get Docker page](https://docs.docker.com/get-started/get-docker/) and follow the instructions for installing Docker Desktop for your operating system. Once it's installed, open it up. It needs to be running in the background but then Scaffold Stellar will handle the rest.

### Scaffold Stellar

Lastly, we'll install the Scaffold Stellar plugin for the Stellar CLI. We suggest using cargo-binstall to install it, which is a tool for installing Rust binaries.

If you don't have it installed, you can do so with:

<details>
    <summary>Macos or Linux</summary>
    ```bash
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
    ```
</details>

<details>
    <summary>Windows</summary>
```powershell
Set-ExecutionPolicy Unrestricted -Scope Process; iex (iwr "https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.ps1").Content
```
</details>

Then install Scaffold Stellar with:

```bash
cargo binstall -y stellar-scaffold-cli
```

Or if you prefer, you can install it directly with Cargo which will compile it from source:

```bash
cargo install --locked stellar-scaffold-cli
```

## 🏗️ Initialize Your Project

Let's initialize a project. Open your terminal and navigate to the directory where you keep your projects, then type:

```bash
stellar scaffold init --tutorial guessing-game-tutorial
```

:::tip

The `--tutorial` flag will create a new project with a simpler starting template specifically for this tutorial. We'll build up to the final version over the next few steps. If you want the full template with the final version of the contract, plus other examples, follow the [Quick Start](/docs/quick-start.mdx) guide instead.

:::

This creates a new project from our starter template containing everything you need. You can call your project anything you'd like. It will also install all the dependencies we need using `npm`. Then navigate into the created directory and start the development server:

```bash
cd guessing-game-tutorial
npm start
```

This command does two things:

1. Starts the development server for the frontend using Vite.
2. Watches for changes in contract code and rebuilds them automatically using Stellar Scaffold's watch command.

That's it! You have a running application communicating with your local Stellar network to interact with your starter contract. Let's check out how it works.

## 🚀 Open the App

The `npm start` command should tell you it's running at Vite's default port, [http://localhost:5173](http://localhost:5173). Open it up and you should see the home page:

```
Welcome to your app!

...

<GuessTheNumber />
Connect wallet to play the guessing game
```

In order to test out our deployed example contract, we'll need to connect to a wallet.

### 💰 Connect a Wallet

In the top right corner, you'll see a big "Connect" button. Click it. You need to have a [Wallet](https://stellar.org/learn/wallets-to-store-send-and-receive-lumens) in order to interact with the dApp. The modal that opened will show a few options if you don't have one already. We recommend using [Freighter](https://www.freighter.app/).

Once it's installed, we need to connect it to our local network running in Docker. Open the extension, click the menu, and navigate to "Settings," then "Network." Click the "Add custom network" button and enter the following info:

- **Name**: `Local`
- **HORIZON RPC URL**: `http://localhost:8000`
- **SOROBAN RPC URL**: `http://localhost:8000/rpc`
- **Passphrase**: `Standalone Network ; February 2017`
- **Friendbot URL**: `http://localhost:8000/friendbot`
- Check **Allow connecting to non-HTTPS networks**

> ℹ️ The 🌐 icon in the extension lets you switch back and forth between this Local network as well as test and main net.

Now click the dApp's "Connect" button and follow the prompts to let the application communicate with Freighter. If it's successful, you should see your account info in the header along with a new "Fund Account" button and a tag for the current network. Click the "Fund Account" button so we can test some transactions.

Once your wallet balance has some XLM, you should see the "GuessTheNumber" component update with a text box. Go ahead and enter some guesses. Right out of the gate we have nice UI to invoke methods on the contract.

So how does this work?

## 🗂️ Exploring the Project Structure

Open the project in your editor. You will see a generated project structure including these files and folders:

```
.
├── .env
├── Cargo.lock
├── Cargo.toml
├── contracts/
│   └──guess-the-number
│      ├── Cargo.toml
│      └── src
│          ├── lib.rs
│          └── test.rs
├── environments.toml
├── packages/
├── README.md
└── rust-toolchain.toml
```

There are a few more files than the ones listed here, but let's highlight some important ones:

- Rust and [Cargo](https://doc.rust-lang.org/cargo/) configuration:
  - `Cargo.toml`: the project's [manifest](https://doc.rust-lang.org/cargo/reference/manifest.html), containing metadata needed to compile everything and package it up. This is where you can name and version your project as well as list the dependencies you need.
  - `Cargo.lock`: Cargo's [lockfile](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html) with exact info about the project's dependencies. We should not manually edit this file, though we should check it into `git` or other source control.
  - `rust-toolchain.toml`: specifies which version of Rust we're using and what platform we're targeting.
- `contracts/`: holds each smart contract as a separate package in our project's Rust workspace. We only need one for this project, but it's nice to know that we can use the same structure for more complex projects that require multiple contracts. The other example contracts in this folder come from our friends at [OpenZeppelin](https://wizard.openzeppelin.com/stellar).
- `packages/`: holds each smart contract's client and types as a separate package for the project's NPM workspace. These are built by Scaffold Stellar and we should not manually edit them. They'll be used by the frontend.
- `.env`: is where we store environment variables that we'll be used by Scaffold Stellar commands.
- `environments.toml`: This is the Scaffold Stellar secret sauce! This file is where we configure:
  - our project's various _environments_, ...
  - which _networks_ are used by each environment, ...
  - all in service of which _contracts_ our project depends on in each of those environments.

So how do all these pieces work together? Here's what Scaffold Stellar handles for you:

1. Our `npm start` command runs `stellar scaffold watch --build-clients`
2. Our `.env` file set an environment variable to say we're in our _development_ environment (`STELLAR_SCAFFOLD_ENV=development`)
3. Scaffold Stellar looked to `environments.toml` for the development environment's configuration, which told it to:
   - Start up a local Stellar network
   - Create an account on the network
   - Build the contracts
   - Deploy them to the network
   - Generate their clients for the frontend

That's a lot of heavy lifting! Normally you'd have to do all this yourself, perhaps in a procedural script, but Scaffold Stellar does it for you. And it's deterministic, meaning you can always reproduce the same results from the same environment configuration. You set configuration values, specifying the desired starting state for your app, and Scaffold Stellar does all the work to get your app into that state.

We deployed the example contract, but we don't even know what it does. Luckily, we built a tool to help with that!

## 🔎 Understand the Contract Code

Go back to your browser and look at the application again. Click the "&lt;/&gt; Debugger" button in the header. These are our contract developer tools. They'll let you explore the contracts available to your application, view documentation, and even run methods to help debug them right from your app!

Select the `guess_the_number` contract and you should see its Contract ID from the local network deployment. You'll also see the contract's documentation for methods like:

- `reset`: Update the number. Only callable by admin.
- `guess`: Guess a number between 1 and 10

This is coming directly from our contract's documentation. Let's open up the initial smart contract code in `contracts/guess-the-number/src/lib.rs` and walk through it.

```rust
#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, BytesN, Env, Symbol};
```

Rust has a great [standard library](https://doc.rust-lang.org/std/) of types, functions, and other abstractions. But our smart contract will run in a constrained WebAssembly environment where the full library isn't available or needed. The `#![no_std]` directive forces us to use only core Rust features.

We can still use explicitly imported features, though, and that's what the next line is doing. Here we're importing some essential items from Stellar's Soroban SDK, and we'll explain each as we get to them. You'll see that many of them replace items from the standard library but are designed for use in Soroban's environment. And the first is `contract`:

```rust
#[contract]
pub struct GuessTheNumber;
```

The `#[...]` syntax in Rust is called an [attribute](https://doc.rust-lang.org/reference/attributes.html). It's a way to label code for the compiler to handle it with special instructions. _Inner_ attributes (with the `#!`) apply to the scope they're within (meaning `!#[no_std]` applies to the whole file/module), and _Outer_ attributes (just the `#`) apply to the next line.

In this case `#[contract]` is an [attribute macro](https://doc.rust-lang.org/reference/procedural-macros.html#attribute-macros), which is a special function called by the compiler that generates code at compile time.

Here we're defining a [struct](https://doc.rust-lang.org/book/ch05-01-defining-structs.html) (a "structure" to hold values) and applying attributes of a Stellar smart contract. A `struct` also allows defining methods. In this case the structs holds no values but we will still define methods on it.

```rust
const THE_NUMBER: &Symbol = &symbol_short!("n");
const ADMIN_KEY: &Symbol = &symbol_short!("ADMIN");
```

Now the most important part of our contract: the number! This line creates a key for storing and retrieving contract data. A `Symbol` is a short string type (max 32 characters) that is more optimized for use on the blockchain. And we're using the `symbol_short` macro for an even smaller key (max 9 characters). As a contract author, you want to use tricks like this to lower costs as much as you can.

The second line creates a key for storing the address of this contract's administrator.

Both of these keys use the `&` which is called a [reference](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html). Instead of the value, it's a pointer to where the value lives.

```rust
#[contractimpl]
impl GuessTheNumber {
```

Let's `impl`ement our contract's functionality.

```rust
pub fn __constructor(env: &Env, admin: Address) {
    Self::set_admin(env, admin);
}
```

A contract's `constructor` runs when it is deployed. In this case, we're saying who has access to the admin functions. We don't want just anyone to be able to reset our number, do we?!

```rust
/// Update the number. Only callable by admin.
pub fn reset(env: &Env) {
    Self::require_admin(env);
    let new_number: u64 = env.prng().gen_range(1..=10);
    env.storage().instance().set(THE_NUMBER, &new_number);
}
```

And here is the reset function. Note that we use `require_admin()` here so only you can run this function. It generates a random number between 1 and 10 and uses our key to store it.

```rust
/// Guess a number between 1 and 10
pub fn guess(env: &Env, a_number: u64) -> bool {
    a_number
        == env
            .storage()
            .instance()
            .get::<_, u64>(THE_NUMBER)
            .expect("no number set")
}
```

Finally, we add the `guess` function which accepts a number as the guess and compares it to the stored number, returning the result. Notice we're using our defined key (that small Symbol) to find stored data that may or may not be there. The thing returned from `get` is a Rust [Option](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html#the-option-enum-and-its-advantages-over-null-values), which is Rust's improvement over the `null` type. An `Option` can be `Some` or `None`. We use [`expect()`](https://doc.rust-lang.org/std/option/enum.Option.html#method.expect) to return the value contained in the `Some` or to panic with the "no number set" message if `None`. We'll talk more about `Option` values later in the tutorial.

```rust
mod test;
```

This line makes sure to include the test module in the file. It's handy to write unit tests for our code in a separate file (`contracts/guess-the-number/src/test.rs`), but you could also write them inline if you want by defining the module. Note you also need to tell the compile that this is a test module, which is at the top of our file `#![cfg(test)]`.

```rust
#[cfg(test)]
mod test {
///
}
```

### 👷 Let's Make a Change

We should still have our original `npm start` command running. I told you it did a lot of heavy lifting for you, but it also updates all of that automatically whenever you make changes to your code. Let's test it out by making a small change and watch the dev server update immediately.

The docstring for our `guess` function says to guess a number "between 1 and 10". But does that include "10"? Let's clarify:

```rust
  /// Guess a number between 1 and 10, inclusive
  pub fn guess(env: &Env, a_number: u64) -> bool {
```

Save the file and watch your terminal output. The contracts get rebuilt, redeployed, and clients for them get regenerated for your frontend. Then Vite hot-reloads your app and you should see the change in the Debugger in your browser.

Tada!

## 🔎 Understand the Application Code

The app's home page uses the `<GuessTheNumber />` component, so we can start by looking at that file in `src/components/GuessTheNumber.tsx`:

```ts
export const GuessTheNumber = () => {
  const [guessedIt, setGuessedIt] = useState<boolean>();
  const [theGuess, setTheGuess] = useState<number>();
  const { address } = useWallet();

  if (!address) {
    return (
      <Text as="p" size="md">
        Connect wallet to play the guessing game
      </Text>
    );
  }
```

We're storing some state for tracking the input's value and whether the guess was successful or not. And we're also using our custom `useWallet` hook to connect to the user's wallet and get their address. This is how we know whether or not you connected to Freighter.

```ts
const submitGuess = async () => {
  if (!theGuess || !address) return;
  const { result } = await game.guess({
    a_number: BigInt(theGuess),
    guesser: address,
  });
  setGuessedIt(result);
};
```

Next, we create a function to handle the user's submission. Hey! Look at that! It's one of our contract's methods right in our TypeScript code: `game.guess()`. Let's follow that import and look at `src/contracts/guess_the_number.ts`.

```ts
import * as Client from "guess_the_number";
import { rpcUrl } from "./util";

export default new Client.Client({
  networkPassphrase: "Standalone Network ; February 2017",
  contractId: "CBPAPSB7SXM3MNJVLXPSD6BRQ2ZN33OQVYWO45332TOP4PQLMCHJV4QN",
  rpcUrl,
  allowHttp: true,
  publicKey: undefined,
});
```

This is the generated RPC client that Scaffold Stellar built for us. It allows us to call methods on the contract and even understand the types for their arguments and return values. You won't ever have to change this file, or the `Client` class in the `/packages` directory.

All you have to do is the fun part, focus on building your application instead of fussing about with all the details of how to get your application to talk to your contracts.

## Summary

That covered a lot, but let's summarize how simple it actually was:

1. We ran `stellar scaffold init guessing-game-tutorial` to generate a project from a starter template
2. We ran `npm start` to build and deploy the contracts to our local network, then run the application
3. We saw the application running in our browser and how it reacted and rebuilt everything anytime we changed the code

That's it! Scaffold Stellar does all the heavy lifting, letting you jump right in to the fun parts of developing your contract and applications. 🎉

## What's Next?

That's a good start, but there's a lot we can improve on. In the next step, we'll:

- Improve the contract code to make it more robust
- Learn about private contract methods
- Practice debugging and handling errors
- Write tests to make sure our contract code is sound

That will give you a better sense of the typical development workflow for contracts and dApps.
