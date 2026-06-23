# Cross-Contract Interaction Framework

This document describes the cross-contract interaction framework implemented in the AxionVera Vault contract, providing a reusable, safe, and standardized way to interact with external Soroban contracts.

## Overview

The framework consists of the `CrossContractClient` struct in `contracts/vault-contract/src/cross_contract.rs` that provides:
- Standardized contract calls
- Error handling
- Validation checks
- Type-safe token interactions

## Core Components

### CrossContractClient Struct

This struct provides static methods for interacting with external contracts:

#### Methods

##### `call<T: IntoVal<Env, Val>>(e: &Env, contract_address: &Address, method: &str, args: &[Val]) -> CrossContractResult<T>`
- Generic method for calling any contract function
- Validates that the target contract is not self
- Returns `CrossContractResult<T>` with proper error handling

##### `token_transfer(e: &Env, token_address: &Address, from: &Address, to: &Address, amount: i128) -> CrossContractResult<()>`
- Type-safe method for transferring tokens using Soroban's standard token interface
- Wraps the `token::Client::transfer` method

##### `token_balance(e: &Env, token_address: &Address, address: &Address) -> CrossContractResult<i128>`
- Type-safe method for getting token balances
- Wraps the `token::Client::balance` method

##### `validate_contract_exists(e: &Env, contract_address: &Address) -> CrossContractResult<()>`
- Validates that a contract address is not the current contract
- Can be extended to perform additional validation checks

## Error Handling

Errors are handled via the `VaultError::CrossContractCallFailed` variant, which is returned when any cross-contract interaction fails.

## Usage Examples

### Basic Contract Call

```rust
use crate::cross_contract::CrossContractClient;

// Call a method on another contract
let result: i64 = CrossContractClient::call(
    &e,
    &other_contract_address,
    "some_method",
    &[arg1.into_val(&e), arg2.into_val(&e)],
)?;
```

### Token Transfer

```rust
CrossContractClient::token_transfer(
    &e,
    &token_address,
    &sender,
    &recipient,
    1000,
)?;
```

### Token Balance Check

```rust
let balance = CrossContractClient::token_balance(
    &e,
    &token_address,
    &user_address,
)?;
```

## Integration in Vault Contract

All token transfers and balance checks in the vault contract now use the `CrossContractClient` instead of directly using `token::Client`, ensuring consistency and safety.

## Testing

Integration tests for the cross-contract framework are included in `contracts/vault-contract/src/test.rs` under the "Cross-Contract Interaction Tests" section.
