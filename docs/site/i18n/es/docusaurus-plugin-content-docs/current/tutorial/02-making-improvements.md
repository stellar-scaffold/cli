---
sidebar_label: Making Improvements
---

# Making Some Basic Improvements

In our initial version, we had a problem: the `guess` function would crash if no number was set yet. Let's fix this by improving how our contract initializes and by creating reusable code for number generation.

## What We'll Accomplish

By the end of this step, you'll have:

- A contract that sets a number immediately upon deployment
- A private helper function for generating random numbers
- A more robust `reset` function that uses our helper
- Better error handling in the `guess` function

## 🪲 Let's break the app!

To understand the bug in our code, let's trigger it. We'll do this by making a small change in `environments.toml`. On our way to finding the line we need to change, we'll learn more about how `environments.toml` works.

Open up `environments.toml` in your editor. Put it side-by-side with the output from `npm run start`. We'll walk through it bit by bit.

### 1. The Network

At the top, you'll see settings for the development network:

```toml
```toml
[development.network]
rpc-url = "http://localhost:8000/rpc"
network-passphrase = "Standalone Network ; February 2017"
run-locally = true
```

Every Stellar network is identified by a `network-passphrase`; it's like the fingerprint of the network and helps keep transactions cryptographically secure between networks. And you connect to any given Stellar network via a configurable `rpc-url`. If you look at the rest of `environments.toml`, every environment's network requires these settings. For our development environment, we also want to run the network locally. `run-locally` tells Scaffold CLI to use _Stellar_ CLI to run a local network container (`stellar container start`) and wait for it to finish startup before moving on to parse the rest of the `development` settings.

These settings correspond to the following `npm run start` output:

```
[0] ℹ️ Starting local Stellar Docker container...
[0] ℹ️ Starting local network
[0] ℹ️ Using network at http://localhost:8000/rpc
```

### 2. The Accounts

Next you'll see this:

```toml
[[development.accounts]]
name = "me"
default = true
```

The double brackets, `[[ ... ]]`, are one way to [make an array in toml](https://toml.io/en/v1.0.0-rc.2#array-of-tables). The snippet above tells Scaffold CLI to use Stellar CLI to generate a keypair for an account named "me" (`stellar keys generate me`) and set this account as the default for all transactions to follow.

If you wanted to create another named account/keypair to use throughout the rest of `environments.toml`, you could do so by adding another `[[development.accounts]]` block:

```toml
[[development.accounts]]
name = "alice"
```

If you look at the `npm run start` output again, this is the corresponding output:

```
[0] ℹ️ Creating keys for "me"
[0] ✅ Key saved with alias me in "~/.config/stellar/identity/me.toml"
[0] ✅ Account me funded on "Standalone Network ; February 2017"
```

On subsequent runs, the key will already exist and the account will already be funded, and the output will tell you so.

### 3. The Contracts (aka "The Contract Clients")

This is what it's all about! You can think of everything in `environments.toml` as existing to configure contract clients.

Here's what that means: your frontend app relies on contracts. Depending on which version of your frontend you are using, those contracts will live on different networks. When you're working in your development environment, you probably want to use the local network (as configured in Scaffold Stellar by default). When you are ready to share an early, staging build of your app with others, you will probably use contracts deployed on Stellar's testnet. When you deploy your production app, you will make calls to mainnet contracts.

Scaffold Stellar encourages you to build separate versions of your frontend for each of these environments. And for each, you specify the contracts you rely on.

:::tip But wait. Isn't the behavior of a given contract the same across different networks? 🤔🤔🤔

If you think about the lifecycle of a contract like our Guess The Number game, you might imagine finalizing the contract, then deploying the exact same contract to your local network, to testnet, and even eventually to mainnet. Why does Scaffold Stellar and `environments.toml` make you specify the contract for each? Why does it rebuild the contract clients for each, as if they might be entirely different? Couldn't we just generate the contract client once, and then change the RPC URL and Network Passphrase that the client gets instantiated with? Same _behavior_, different networks & contracts?

In theory, this sounds reasonable. In practice, contracts rarely have the same exact implementation across different networks. Your local contract will have all the latest changes; it will be like your `main` branch or a nightly build. Messy, fast-paced, experimental. Your staging contract will be like a `beta` release—it will have stuff you haven't yet pushed to your main app. And even more, you could add feature flags to permanently ship different versions of your contract to staging and mainnet. Imagine a contract that adds admin backdoors in staging, but strips them out in production.

Scaffold Stellar wants to help you avoid bugs in all these situations. The contract clients are rebuilt for each environment, and they're built _in strict TypeScript_. So if you worked locally on a cool new feature with a smart contract method `my_cool_new_method`, and your frontend makes unguarded calls to this, then your frontend build for staging and production will fail, because those contracts don't implement `my_cool_new_method`.

:::

For staging and production, these must be live, deployed contracts. But in development, you are likely working on your contracts at the same time as your frontend! So the `development.contracts` handling has some allowances, some superpowers, that `staging.contracts` and `production.contracts` lack. Let's see:

```toml
[development.contracts.guess_the_number]
client = true

constructor_args = """
--admin me
"""

after_deploy = """
reset
"""
```

This is a [Toml table](https://toml.io/en/v1.0.0-rc.2#table). See the TOML spec for other ways you could specify the same information.

Let's walk through this line by line:

- `[development.contracts.guess_the_number]`: this project only has one contract, so we can specify the settings for its contract clients here. You could also have a `[development.contracts]` with a more JSON-like specification for `guess_the_number` (`guess_the_number = { client = true, … }`).
- `guess_the_number`: this name must match the name of the contract specified in its `Cargo.toml` file, but in underscore-case. Compare it to the `name` field in `contracts/guess-the-number/Cargo.toml` and the generated Wasm files (`ls target/wasm32v1-none/release/*.wasm`).

   The `npm run start` output that corresponds to this came out right at the top:

   ```
   [0] ℹ️ Watching …/guessing-game-tutorial/contracts/guess-the-number
   ```
- `client = true`: this tells Scaffold CLI to generate a contract client for this contract.

   This results in the `npm run start` output:

   ```
   [0] ℹ️ Binding "guess_the_number" contract
   [0] ✅ 'npm run build' succeeded in …/guessing-game-tutorial/target/packages/guess_the_number
   ```

   The contract _client_ also gets called the "TS (or TypeScript) Bindings" for the contract, because they are generated with the Stellar CLI command `stellar contract bindings typescript`.

- `constructor_args`: the contract has a `constructor`, as we saw in the previous step. This `constructor_args` setting specifies the arguments to use when deploying & initializing the contract. You could deploy the contract yourself with:

   ```bash
   stellar contract deploy \
       --wasm-hash [find this in npm run start output] \
       --source me \
       -- \
       --admin me
   ```

   As you can see, the `constructor_args` get passed directly along to this `stellar contract deploy` command.

   `client = true` and the `constructor_args` settings together resulted in this `npm run start` output:

   ```
   [0] ℹ️ Installing "guess_the_number" wasm bytecode on-chain...
   [0] ℹ️   ↳ hash: d801a98511519b2e9d4f2fadffc4215fc81f91426381dbdb328d10252e8298ac
   [0] ℹ️ Instantiating "guess_the_number" smart contract
   [0] ℹ️   ↳ contract_id: CCMMU6UYIPGSBR7ZP4DTQEEOQDHL3PJ52ZD7FJIFG4O46Q3QPVGVHAAV
   ```

   The contract gets deployed in two steps:

   1. The Wasm gets uploaded to the blockchain, so that many contracts could use it.
   2. A contract gets deployed (aka "instantiated", in the current parlance of this output) so that there is an actual smart contract that refers to, or points to, that Wasm.

- `after_deploy`: calls to the contract to make after it gets deployed. Kind of like the `constructor_args`, these are specified using _only_ the part that comes after the `--` (this part of the command is sometimes called the "slop", so these `after_deploy` scripts are _slop only!_). The setting above tells Scaffold CLI to make the following call, after deploying the contract:

   ```bash
   stellar contract deploy \
       --id guess_the_number
       --source me
       -- \
       reset
   ```

   This `after_deploy` script produces this `npm run start` output:

   ```
   [0] ℹ️ Running after_deploy script for "guess_the_number"
   [0] ℹ️   ↳ Executing: stellar contract invoke --id CCMMU6UYIPGSBR7ZP4DTQEEOQDHL3PJ52ZD7FJIFG4O46Q3QPVGVHAAV --config-dir /Users/chadoh/code/scast/frontend -- reset
   [0] ℹ️   ↳ Result: Res("")
   ```

### Let's break it already!

That's it! That last line! That's how we break things. Go ahead and remove the `after_deploy` script entirely.

```diff
 constructor_args = """
 --admin me
 """
-
-after_deploy = """
-reset
-"""
```

Can you guess what will happen?

If you already tried re-running the `guess` logic in the app, you'll see...

Nothing. Nothing happens. At least not yet.

The contract didn't change, so Scaffold CLI didn't re-deploy the contract. You're still using the one that had the `reset` method called right after deploy.

Once [theahaco/scaffold-stellar#259](https://github.com/theahaco/scaffold-stellar/issues/259) is complete, you will be able to run `stellar scaffold reset`. Until then, you can remove the alias that Scaffold Stellar uses to keep track of this contract. Stop the `npm run start` process, then run:

```bash
stellar contract alias remove guess_the_number --network local
```

Re-run `npm run start` and you'll see it churn through re-deploying the contract. This time you won't see the output about running the `after_deploy` script.

Now you can trigger the bug in two exciting ways!

1. Go to the Debugger page and submit a `guess`. 💥 BOOM! In the Response box, you'll see:

  ```
  Simulation Failed
  HostError: Error(WasmVm, InvalidAction) Event log (newest first): 0: [Diagnostic Event] contract:CCW3B3N6HHG2TVAHUGJUU6TJD3AXTAJN35TYUNFZ4X6D2Y4JCGVJLD7K, topics:[error, Error(WasmVm, InvalidAction)], data:["VM call trapped: UnreachableCodeReached", guess] 1: [Diagnostic Event] topics:[fn_call, CCW3B3N6HHG2TVAHUGJUU6TJD3AXTAJN35TYUNFZ4X6D2Y4JCGVJLD7K, guess], data:1 `
  ```

2. Go to the home page, make sure your browser's inspector console is open. Then find the &lt;GuessTheNumber /&gt; section and submit a guess. 💥 BOOM! You should see a similar error in your browser console.

## Fixing the Problem

In our current contract, the `__constructor` only sets the admin, but doesn't set an initial number. This means:

1. If someone calls `guess` before `reset`, it will crash! With a really ugly error.
2. The number generation logic is only in `reset`, making it hard to reuse.

Isn't it silly, though, that the admin needs to call `reset` before the contract can be used? We already have a `__constructor`, let's use it!

We'll do this in two steps:

1. Move the initial number-setting logic to a helper function
2. Call this helper from both `__constructor` and `reset`
3. Bonus: go Pro Mode and save bytes with `unsafe`

### Step 1: 🔒 Create a Private Helper Function

First, let's extract the number generation into a private helper function. This follows the DRY principle (Don't Repeat Yourself) and makes our code more maintainable.

Open `contracts/guess-the-number/src/lib.rs` and add this private function inside the `impl GuessTheNumber` block:

```rust
#[contractimpl]
impl GuessTheNumber {
    // ... existing functions ...

    /// Private helper function to generate and store a new random number
    fn set_random_number(env: &Env) {
        let new_number: u64 = env.prng().gen_range(1..=10);
        env.storage().instance().set(&THE_NUMBER, &new_number);
    }
}
```

#### Understanding Private Functions

Notice that this function doesn't have `pub` in front of it - this makes it private. Private functions:

- Can only be called from within the same contract
- Don't become part of the contract's public API
- Are useful for internal logic and code reuse
- Help keep your contract interface clean and focused

### Step 2: 👷‍♂️ Update the Constructor

Now let's modify the `__constructor` to set an initial number when the contract is deployed:

```rust
pub fn __constructor(env: &Env, admin: &Address) {
    Self::set_admin(env, admin);
    Self::set_random_number(env); // Add this line
}
```

#### Why This Improves Things

By setting a number in the constructor:

1. **Immediate functionality**: The contract works right after deployment
2. **No crash risk**: `guess` will never encounter a missing number
3. **Better user experience**: Players can start guessing immediately

Let's also simplify our `reset` function to use the new helper:

```rust
/// Update the number. Only callable by admin.
pub fn reset(env: &Env) {
    Self::require_admin(env);
    Self::set_random_number(env);
}
```

Much cleaner! The logic is now centralized in our helper function. Note that this is still a public function, see the `pub`? The distinction between "public" and "private" might seem confusing here. Let's run the application and it should clear everything up:

```bash
$ npm start
```

Click over to `&lt;/&gt; Debugger` if you're not there already and select the `guess_the_number` contract. You'll see that `reset` is listed here, but `set_random_number` is not.

Our `reset` method is available to be called by code _outside_ our contract because we opted in to it being a public method with the `pub` keyword. Our `set_random_number` is private by default, it's not visible to the outside world. It's not listed in the Contract Explorer. It's not listed in the CLI help either:

```bash
$ stellar contract invoke --id guess_the_number --source me --network local -- help
Commands:
  reset    Update the number. Only callable by admin.
  guess    Guess a number between 1 and 10
  upgrade  Upgrade the contract to new wasm. Only callable by admin.
  help     Print this message or the help of the given subcommand(s)
```

It would error if you tried to invoke it:

```bash
$ stellar contract invoke --id guess_the_number --source me --network local -- set_random_number
error: unrecognized subcommand 'set_random_number'
```

#### Wait, So Anyone Can Call Reset?

Nope! Just because we made it public, we still require authentication so only admins can call it. Rust's idea of public vs private handles "where" the functions can be called. You still need to handle "who" calls it. That's why we set the contract admin in it's constructor method and check it with `Self::require_admin(env);`.

You can try this out by invoking it from the Contract Explorer in your browser. The admin is `me`, but you didn't import that account into your browser wallet. Go ahead and hit `Submit` on the `reset` function.

You could also try this out in the CLI. Create a non-admin identity to see how it fails:

```bash
$ stellar keys generate bob --network local --fund
✅ Key saved with alias bob in ".config/stellar/identity/bob.toml"
✅ Account bob funded on "Standalone Network ; February 2017"

$ stellar contract invoke --id guess_the_number --source bob --network local -- reset
❌ error: Missing signing key for account GDAQWVA6REGN47BBCFY6SGQ4YTIGMDZZFHDOVUZXMVRAAT6OEZGCACGH
```

The account called `me` is the admin, Bob is just a regular user. `me` can call `reset`, Bob gets an error.

### Bonus Step 3: Go Pro and Save Bytes with `unsafe`

Let's look at that `expect` line again:

```rust
pub fn guess(env: &Env, a_number: u64) -> bool {
    a_number
        == env
            .storage()
            .instance()
            .get::<_, u64>(&THE_NUMBER)
            .expect("no number set")
}
```

You _know_ now, beyond any doubt, that your contract will _always_ store a number. The `expect` will _never_ encounter a `None`, and will never panic with the "no number set" error. This `expect` logic and the 13 characters inside the "no number set" string are just wasted space in your contract!

Sure, it's not a _lot_ of wasted space. But every time a user invokes your contract, they will need to pay for the contract's Wasm bytecode to be deserialized from blockchain storage, loaded into the Stellar runtime, and executed. Over the lifetime of your contract and the blockchain, it all adds up!

You can tell Rust that you know what you're doing here to get rid of this waste.

```rust
pub fn guess(env: &Env, a_number: u64) -> bool {
    a_number == unsafe { env.storage().instance().get(THE_NUMBER).unwrap_unchecked() }
}
```

Or, if you want to clean things up a little and add some comments about why it's ok to use `unsafe` (a good idea!), you could do this:

```rust
pub fn guess(env: &Env, a_number: u64) -> bool {
    a_number == Self::number(env)
}

/// readonly function to get the current number
fn number(env: &Env) -> u64 {
    // We can unwrap because the number is set in the constructor
    // and then only reset by the admin
    unsafe { env.storage().instance().get(THE_NUMBER).unwrap_unchecked() }
}
```

## Your Complete Updated Contract

Here's what your `lib.rs` should look like now:

```rust
#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Address, BytesN, Env, Symbol};

#[contract]
pub struct GuessTheNumber;

const THE_NUMBER: Symbol = symbol_short!("n");
pub const ADMIN_KEY: &Symbol = &symbol_short!("ADMIN");

#[contractimpl]
impl GuessTheNumber {
    pub fn __constructor(env: &Env, admin: &Address) {
        Self::set_admin(env, admin);
        Self::set_random_number(env);
    }

    /// Update the number. Only callable by admin.
    pub fn reset(env: &Env) {
        Self::require_admin(env);
        Self::set_random_number(env);
    }

    /// Guess a number between 1 and 10
    pub fn guess(env: &Env, a_number: u64) -> bool {
        a_number == Self::number(env)
    }

    /// Private helper function to generate and store a new random number
    fn set_random_number(env: &Env) {
        let new_number: u64 = env.prng().gen_range(1..=10);
        env.storage().instance().set(&THE_NUMBER, &new_number);
    }

    /// readonly function to get the current number
    fn number(env: &Env) -> u64 {
        // We can unwrap because the number is set in the constructor
        // and then only reset by the admin
        unsafe { env.storage().instance().get(THE_NUMBER).unwrap_unchecked() }
    }

    /// Upgrade the contract to new wasm. Only callable by admin.
    pub fn upgrade(env: &Env, new_wasm_hash: BytesN<32>) {
        Self::require_admin(env);
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    /// Get current admin
    pub fn admin(env: &Env) -> Option<Address> {
        env.storage().instance().get(ADMIN_KEY)
    }

    /// Set a new admin. Only callable by admin.
    fn set_admin(env: &Env, admin: &Address) {
        // Check if admin is already set
        if env.storage().instance().has(ADMIN_KEY) {
            panic!("admin already set");
        }
        env.storage().instance().set(ADMIN_KEY, admin);
    }

    /// Private helper function to require auth from the admin
    fn require_admin(env: &Env) {
        let admin = Self::admin(env).expect("admin not set");
        admin.require_auth();
    }
}

mod test;
```

## Step 6: 🧪 Test Your Improvements

Let's test that our improvements work. You should still have the `npm start` process running from earlier. If not, run it again and we can look a little closer at what it's doing. There's two concurrent processes:

1. `stellar scaffold watch --build-clients`: watches for any changes in your `contracts/` folders, then rebuilds and redeploys them
2. `vite`: watches for any changes in your `src/` folder and hot-reloads the UI

That means any time you add a method, tweak arguments, or even add documentation, everything is immediately reflected on the local network, your application in the browser, and in the Contract Explorer. Let's add some info to the `guess` method's documentation:

```rust
    /// Guess a number between 1 and 10, inclusive. Returns a boolean.
    pub fn guess(env: &Env, a_number: u64) -> bool {
```

As soon as you hit save, watch the Contract Explorer reload with the new text. Nifty, right? This massively speeds up your development time. But we can go even further.

### How to Write Unit Tests

TODO

## What We've Learned

In this step, we covered several important concepts:
1. `environments.toml` structure
  - **network**: Configure which network each enviroment connects to, and automatically run a local node
  - **accounts**: Create account keypairs for an environment
  - **contracts**: Specify "contract dependencies," for which to build contract clients, for each environment. In `development`, Scaffold CLI will also automatically build & deploy the contracts, too, with an optional `after_deploy` script
2. Code Organization
	- **Private functions**: Help organize code and prevent external access to internal logic
	- **DRY principle**: Don't repeat yourself - extract common logic into reusable functions
3. Contract Lifecycle
	- **Immediate functionality**: Contracts should work right after deployment
	- **Consistent state**: Always ensure your contract is in a valid state

Our contract is now much more robust:

- ✅ Works immediately after deployment
- ✅ Clean, reusable code structure
- ✅ Better error handling
- 🚫 Still no authentication (anyone can guess)
- 🚫 Still no payments (how do you win the prize? what prize?)

## What's Next?

In the next step, we will:

- Convert `guess` from a view method to a change method
- Make the admin fund the pot when calling `reset`
- Require users to pay a small amount of XLM per-guess
- Reward correct guesses

Finally, the economic incentives that blockchains are all about! Let's go.
