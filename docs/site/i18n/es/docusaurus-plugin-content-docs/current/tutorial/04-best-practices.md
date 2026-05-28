---
sidebar_label: Best Practices
---

# Best Practices For Production Contracts

Our guessing game works, but it's not quite ready for production. Let's add professional-grade error handling, security measures, and some clever techniques to make the stored number less obvious to observers.

## What We'll Accomplish

By the end of this step, you'll have:
- Custom error types instead of panics
- Robust error handling throughout the contract
- Better security against common attacks
- Obfuscated number storage
- Event logging for monitoring
- Production-ready smart contract code

## Understanding Current Problems

Our current contract has several issues:

1. **Panic-prone**: Uses `unwrap()` and `expect()` which crash the contract
2. **Visible number**: The secret number is stored in plain sight on-chain
3. **Poor error reporting**: Generic error messages don't help users
4. **No events**: Hard to monitor what's happening
5. **Basic validation**: Doesn't check for edge cases

Let's fix these systematically!

## Step 1: Define Custom Error Types

First, let's create proper error types. Add this after your imports:

```rust
#![no_std]
use admin_sep::{Administratable, Upgradable};
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token,
    Address, Env, Symbol
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum Error {
    /// Number has not been set yet
    NoNumberSet = 1,
    /// Guess must be between 1 and 10
    InvalidGuess = 2,
    /// Insufficient balance to pay guess fee
    InsufficientBalance = 3,
    /// Prize pot is empty
    EmptyPot = 4,
    /// Token transfer failed
    TransferFailed = 5,
}
```

### Understanding Custom Errors

Custom errors provide several benefits:
- **Better UX**: Users get meaningful error messages
- **No crashes**: Contract returns errors instead of panicking
- **Debuggability**: Developers can handle different error cases
- **Professional**: Shows attention to detail and robustness

## Step 2: Add Events for Monitoring

Let's add events so we can monitor what's happening in our contract:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[contracttype]
pub enum Error {
    // ... error definitions
}

#[contracttype]
pub struct GameResetEvent {
    pub admin: Address,
    pub new_pot: u64,
    pub funding_amount: u64,
}

#[contracttype]
pub struct GuessEvent {
    pub guesser: Address,
    pub guess: u64,
    pub is_correct: bool,
    pub pot_before: u64,
    pub winnings: u64,
}
```

Events help with:
- **Monitoring**: Track game activity
- **Analytics**: Understand player behavior
- **Debugging**: See what happened in past transactions
- **Frontend updates**: Real-time UI updates

## Step 3: Add Number Obfuscation

_🏗️✨ TODO: deploy to testnet and use contract explorer to see number in plain text_

Let's make the stored number less obvious by combining it with a salt. We'll have to store it alongside the number:

```rust
const THE_NUMBER: Symbol = symbol_short!("n");
const NUMBER_SALT: Symbol = symbol_short!("salt");    // Add this line
```

Now let's add functions to handle obfuscated storage:

```rust
/// Private helper function to generate and store a new random number with obfuscation
fn set_random_number(env: &Env) {
    let new_number: u64 = env.prng().gen_range(1..=10);
    let salt: u64 = env.prng().gen();

    // Store the number XORed with the salt
    let obfuscated = new_number ^ salt;
    env.storage().instance().set(&THE_NUMBER, &obfuscated);
    env.storage().instance().set(&NUMBER_SALT, &salt);

    // etc...
}

/// Private helper to retrieve the actual number
fn get_actual_number(env: &Env) -> Result<u64, Error> {
    let obfuscated = env.storage()
        .instance()
        .get::<_, u64>(&THE_NUMBER)
        .ok_or(Error::NoNumberSet)?;

    let salt = env.storage()
        .instance()
        .get::<_, u64>(&NUMBER_SALT)
        .ok_or(Error::NoNumberSet)?;

    Ok(obfuscated ^ salt)
}
```

XOR (exclusive or) is a simple but effective obfuscation technique:
- `number ^ salt = obfuscated_value`
- `obfuscated_value ^ salt = number` (XOR is reversible)
Without knowing the salt, the stored value looks random. Note this doesn't provide cryptographic security, but makes casual observation much harder.


### Advanced techniques

_🏗️✨ TODO: add explanations_

- commit-reveal pattern
- off-chain oracle
- time-delayed reveal

## Step 4: Update Functions with Error Handling

_🏗️✨ TODO:  clean up code and test_

Let's update our main functions to use proper error handling:

```rust
/// Update the number and fund the prize pot. Only callable by admin.
pub fn reset(env: &Env, admin_funding: u64) -> Result<(), Error> {
    Self::require_admin(env);

    // Validate funding amount
    if admin_funding == 0 {
        return Err(Error::EmptyPot);
    }

    // Generate new number
    Self::set_random_number(env);

    // Get admin address
    let admin = Self::get_admin(env);

    // Get current pot before adding funding
    let current_pot = Self::get_prize_pot(env);

    // Transfer funding from admin to contract
    let native_token = Self::get_native_token(env);
    let token_client = token::Client::new(env, &native_token);

    // This could fail, so we need to handle it
    token_client.transfer(&admin, &env.current_contract_address(), &(admin_funding as i128));

    // Add to prize pot
    let new_pot = current_pot + admin_funding;
    env.storage().instance().set(&PRIZE_POT, &new_pot);

    // Emit event
    env.events().publish(
        (symbol_short!("reset"),),
        GameResetEvent {
            admin: admin.clone(),
            new_pot,
            funding_amount: admin_funding,
        },
    );

    Ok(())
}
```

Now the improved `guess` function:

```rust
/// Guess a number between 1 and 10
/// Costs a fee and pays out the entire pot if correct
pub fn guess(env: &Env, guesser: Address, a_number: u64) -> Result<bool, Error> {
    // Verify the guesser is actually the one calling this function
    guesser.require_auth();

    // Validate guess is in range
    if a_number < 1 || a_number > 10 {
        return Err(Error::InvalidGuess);
    }

    // ...

    // Emit event
    env.events().publish(
        (symbol_short!("guess"),),
        GuessEvent {
            guesser: guesser.clone(),
            guess: a_number,
            is_correct,
            pot_before: prize_pot,
            winnings,
        },
    );

    Ok(is_correct)
}
```

## Step 5: Admin Functions?

_🏗️✨ TODO: is this necessary?_


Here's your complete, production-ready contract:

_🏗️✨ TODO: add link to github repo of sample project_

## Step 7: Test the Secure Contract

Let's test all our new security and error handling features:

### Test Error Handling

Let's test that our error handling works:

```bash
# Test invalid guess (should return error, not crash)
stellar contract invoke \
  --id [CONTRACT_ID] \
  --source bob \
  --network local \
  -- guess \
  --guesser $(stellar keys address bob) \
  --a_number 15
```

You should get a proper error message instead of a crash!

### Test Contract Info

```bash
stellar contract invoke \
  --id [CONTRACT_ID] \
  --source alice \
  --network local \
  -- get_contract_info
```

This should return `[0, 100000, true]` showing: pot=0, fee=100k stroops, number is set.

### Test the Event System

When you make guesses and resets, you should see events in the transaction response. These events can be monitored by frontend applications for real-time updates.

### Test Emergency Functions

```bash
# First fund the contract
stellar contract invoke \
  --id [CONTRACT_ID] \
  --source alice \
  --network local \
  -- reset --admin_funding 5000000

# Test emergency withdraw (admin only)
stellar contract invoke \
  --id [CONTRACT_ID] \
  --source alice \
  --network local \
  -- emergency_withdraw
```

## Step 8: Gas Optimization?

_🏗️✨ TODO_

Our current contract is quite efficient, but for even better performance:
- Use `Temporary` storage for short-lived data
- Batch operations when possible
- Avoid unnecessary storage reads

## What We've Accomplished

Our contract has evolved from a simple demo to production-ready code:

### Security Improvements
- ✅ **Custom error types**: No more crashes, proper error handling
- ✅ **Input validation**: Check all user inputs
- ✅ **Obfuscated storage**: Secret number is not obvious
- ✅ **Emergency functions**: Admin can recover funds if needed
- ✅ **Event logging**: Full audit trail of all actions

### Code Quality
- ✅ **Proper error propagation**: Using `Result<T, Error>` throughout
- ✅ **Defensive programming**: Check all assumptions
- ✅ **Clean separation**: Private helpers for internal logic
- ✅ **Documentation**: Clear function purposes and error cases

### User Experience
- ✅ **Meaningful errors**: Users know exactly what went wrong
- ✅ **Real-time events**: Frontend can show live updates
- ✅ **Admin tools**: Easy management and monitoring
- ✅ **Reliability**: Contract won't crash under normal or edge cases

## Production Deployment Checklist

Before deploying to mainnet, ensure:

1. **Testing**: Comprehensive test suite covering all edge cases
2. **Security audit**: Have the contract reviewed by security experts
3. **Gas optimization**: Minimize transaction costs
4. **Monitoring**: Set up event monitoring and alerting
5. **Admin procedures**: Document emergency procedures
6. **User documentation**: Clear instructions for users

## Comparing Our Journey

Let's look at how far we've come:

### Step 1: Basic Contract
- Simple number storage
- Panic-prone code
- No authentication

### Step 2: Development Workflow and a UI
- Constructor initialization
- Private helper functions
- Basic error handling

### Step 3: Adding Transactions
- Proper authorization
- Token transfers
- Financial incentives

### Step 4: Production Ready
- Comprehensive error handling
- Security measures
- Professional monitoring

## Congratulations!

You've built a complete, production-ready smart contract that demonstrates:
- Proper Rust and Soroban patterns
- Economic incentives and token handling
- Professional error handling and security
- Real-world blockchain development practices

This knowledge forms the foundation for building more complex decentralized applications. The patterns you've learned - authentication, token transfers, error handling, and event emission - appear in virtually every serious blockchain application.

Keep building, keep learning, and welcome to the world of decentralized application development!
