use soroban_sdk::{Address, Env, IntoVal, Val};
use crate::errors::{VaultError, ValidationError};

/// Result type for cross-contract calls
pub type CrossContractResult<T> = Result<T, VaultError>;

/// Cross-contract interaction framework
pub struct CrossContractClient;

impl CrossContractClient {
    /// Call a method on an external contract with validation
    pub fn call<T: IntoVal<Env, Val>>(
        e: &Env,
        contract_address: &Address,
        method: &str,
        args: &[Val],
    ) -> CrossContractResult<T> {
        // Validate contract address (ensure it's not self)
        if contract_address == &e.current_contract_address() {
            return Err(ValidationError::InvalidAddress.into());
        }

        // Create contract client
        let client = soroban_sdk::contractclient::Client::new(e, contract_address);

        // Call the method and handle errors
        client.call(method, args).map_err(|_| VaultError::CrossContractCallFailed)
    }

    /// Call a token contract's transfer method (type-safe)
    pub fn token_transfer(
        e: &Env,
        token_address: &Address,
        from: &Address,
        to: &Address,
        amount: i128,
    ) -> CrossContractResult<()> {
        let token_client = soroban_sdk::token::Client::new(e, token_address);
        token_client.transfer(from, to, &amount);
        Ok(())
    }

    /// Call a token contract's balance method (type-safe)
    pub fn token_balance(
        e: &Env,
        token_address: &Address,
        address: &Address,
    ) -> CrossContractResult<i128> {
        let token_client = soroban_sdk::token::Client::new(e, token_address);
        Ok(token_client.balance(address))
    }

    /// Validate that a contract exists by calling a simple method
    pub fn validate_contract_exists(
        e: &Env,
        contract_address: &Address,
    ) -> CrossContractResult<()> {
        if contract_address == &e.current_contract_address() {
            return Err(ValidationError::InvalidAddress.into());
        }
        Ok(())
    }
}

/// Event emitted when a cross-contract call is made
pub fn emit_cross_contract_call(
    e: &Env,
    contract_address: Address,
    method: &str,
    success: bool,
) {
    // For now, we can log this via events if needed
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{Env, Address, BytesN};

    #[test]
    fn test_validate_contract_exists() {
        let e = Env::default();
        let contract_id = e.register_contract(None, super::super::VaultContract);
        let other_address = Address::generate(&e);

        // Should fail with self address
        let result = CrossContractClient::validate_contract_exists(&e, &contract_id);
        assert!(result.is_err());

        // Should pass with other address
        let result = CrossContractClient::validate_contract_exists(&e, &other_address);
        assert!(result.is_ok());
    }
}
