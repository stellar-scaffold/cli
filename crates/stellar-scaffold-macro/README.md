# stellar-scaffold-macro

This crate contains a utility macro for importing Soroban smart contracts. The main functionality is provided through the `import_contract!` macro which generates the necessary client code for interacting with Soroban contracts.

`import_contract_client!` is a [procedural macro](https://doc.rust-lang.org/reference/procedural-macros.html) that automatically generates a contract client for a given contract. It expects the contract name to match either a published contract or a contract in your current workspace. The macro will locate the contract's WASM file and generate the appropriate Rust bindings for interacting with it.

For example:
```rust
import_contract_client!(my_contract);
```

This will generate a module containing the client code needed to interact with `my_contract`.

See lib.rs for the implementation details of the import_contract_client macro.

## Stellar assets

```rust
use stellar_scaffold_macro::import_asset;

import_asset!("native"); // Also accepts "xlm".
import_asset!("USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN");

// Given a Soroban Env:
// let address = native::contract_id(&env);
// let token = native::token_client(&env);
// let admin = USDC::stellar_asset_client(&env);
```

`import_asset!` embeds the asset's serialized identity and derives its Stellar
Asset Contract address at runtime using the executing network. The same compiled
contract can therefore derive addresses on local, testnet, futurenet, mainnet,
and custom networks without rebuilding for a different `STELLAR_NETWORK` or
`STELLAR_NETWORK_PASSPHRASE`.

The generated helpers compute addresses and construct clients; they do not deploy
asset contracts or verify their existence. An issued asset's code and issuer are
still fixed by the macro input. If different networks use different issuers, pass
or store that configuration separately.
