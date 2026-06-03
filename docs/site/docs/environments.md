# Environment Configuration

Scaffold Stellar uses an `environments.toml` file to manage different deployment environments and contract configurations.

## Configuration File Structure

```toml
[development]
network = {
    name = "local",                 # use local network
    run_locally = true              # start up the local docker container
}
accounts = ["account1", "account2"] # Account aliases to create

[staging]
network = {
    name = "testnet",               # Use Stellar testnet
}

[production]
network = {
    name = "mainnet",               # Use Stellar mainnet
}
```

## Network Configuration

Each environment can specify network settings:

```toml
network = {
    name = "<network-name>",           # Optional: Use predefined network (mainnet/testnet/local)
    rpc_url = "<url>",                # Optional: Custom RPC endpoint
    network_passphrase = "<phrase>",   # Optional: Network passphrase
    rpc_headers = [["key", "value"]], # Optional: Custom RPC headers
    run_locally = false               # Optional: Whether to run local network (default: false)
}
```

## Account Configuration

Configure accounts for contract deployment and testing:

```toml
accounts = [
    "account1",                        # Simple account alias
    { name = "admin", default = true } # Account with additional settings
]
```

## Contract Configuration

Configure smart contracts for each environment:

```toml
[development.contracts.my_contract]
client = true                      # Generate TypeScript client (default: true)
constructor_args = """             # Initialization script if needed
    --arg1 param1 --arg2 param2
"""
after_deploy = """                 # contract setup invocation logic for after initial deployment
    STELLAR_ACCOUNT=admin fund --to admin --amount 100
"""

[production.contracts.my_contract]
id = "C..."                        # Contract ID for production/staging
client = true                      # Generate TypeScript client
```

### Configuration Options

#### `client` (boolean, default: true)

- Controls whether a TypeScript client package is generated for this contract
- Set to `false` to skip client generation for utility contracts

```toml
[development.contracts.my_contract]
client = false  # Skip TypeScript client generation
```

#### `id` (string, optional)

- Specifies a fixed contract ID for the contract
- Required in production/staging environments
- Must be a valid Stellar contract ID

```toml
[production.contracts.my_contract]
id = "C..."  # Use specific contract ID
```

#### `constructor_args` (string, optional)

- Arguments passed to contract constructor during deployment
- Executes as part of the deployment transaction
- Single line of space-separated arguments
- Can use `STELLAR_ACCOUNT=<alias>` to specify the deployer account
- Supports command substitution with `$(command)`

```toml
[development.contracts.my_contract]
constructor_args = "--arg1 1000 --account $(stellar keys address admin)"  # Basic args

# With specific deployer account
constructor_args = "STELLAR_ACCOUNT=admin --arg1 value1 --arg2 value2"

# With command substitution
constructor_args = "--account1 $(stellar keys address user1) --account2 $(stellar keys address user2)"
```

#### `after_deploy` (string, optional)

- Initialization script to run after contract deployment
- Only runs in development/testing environments
- Supports multiple commands on separate lines
- Can use `STELLAR_ACCOUNT=<alias>` to specify the source account
- Supports command substitution with `$(command)`

```toml
[development.contracts.my_contract]
after_deploy = """
# Basic initialization
initialize --param1 value1 --param2 value2

# Use specific account
STELLAR_ACCOUNT=admin set_admin --admin "new_admin"

# Command substitution
set_value --value "$(stellar keys address admin)"

# Multiple operations
create_pool --name "Pool A"
add_liquidity --amount 1000
set_fee_rate --rate 0.003
"""
```

### Example Configurations

```toml
# Token contract with constructor args
[development.contracts.token]
client = true
constructor_args = "--name Token --symbol TKN --decimals 8"

# Contract deployed by admin with dynamic arguments
[development.contracts.marketplace]
client = true
constructor_args = "STELLAR_ACCOUNT=admin --treasury-account $(stellar keys address treasury)"

# Contract with both constructor args and after_deploy script
[development.contracts.game]
client = true
constructor_args = "STELLAR_ACCOUNT=admin --name GameV1 --start 1000"
after_deploy = """
    # Additional setup after deployment
    add_player --address "$(stellar keys address player1)"
    set_difficulty --difficulty 3
"""

# Production environment with fixed contract ID
[production.contracts.token]
client = true
id = "CC5YYARE2TSLA..."  # Must be valid contract ID

# Utility contract without client generation
[development.contracts.utils]
client = false

# Complex initialization with multiple accounts
[development.contracts.marketplace]
client = true
after_deploy = """
    # Set up admin
    STELLAR_ACCOUNT=admin set_admin_account --account "$(stellar keys address admin)"

    # Configure fees
    STELLAR_ACCOUNT=admin set_fee_rate --rate 250

    # Add initial listing
    STELLAR_ACCOUNT=seller create_listing --name "Item A" --price 1000
"""
```

## Environment Variables

- `STELLAR_SCAFFOLD_ENV`: Set the current environment (development/testing/staging/production)
- `STELLAR_ACCOUNT`: Default account for transactions (set automatically)
- `STELLAR_RPC_URL`: RPC endpoint URL (set from network config)
- `STELLAR_NETWORK_PASSPHRASE`: Network passphrase (set from network config)

## Usage

1. Create `environments.toml` in your project root
2. Configure environments, networks, and contracts
3. Set `STELLAR_SCAFFOLD_ENV` to choose environment
4. Use `stellar scaffold build` or `stellar scaffold watch` to deploy and generate clients
