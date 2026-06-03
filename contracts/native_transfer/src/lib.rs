#![no_std]

mod errors;
mod events;

use soroban_sdk::{contract, contractimpl, Address, Env};

pub use errors::Error;
pub use events::NativeTransferExecuted;

#[contract]
pub struct NativeTransferContract;

#[contractimpl]
impl NativeTransferContract {
    /// Initialize with the native asset contract address
    ///
    /// # Arguments
    /// * `native_token_address` - The Stellar native XLM token contract address
    ///
    /// # Errors
    /// * `Error::AlreadyInitialized` - called more than once
    pub fn initialize(env: Env, native_token_address: Address) -> Result<(), Error> {
        if env.storage().instance().has(&"native_token") {
            return Err(Error::AlreadyInitialized);
        }

        env.storage()
            .instance()
            .set(&"native_token", &native_token_address);

        Ok(())
    }

    /// Execute a native XLM transfer
    ///
    /// # Arguments
    /// * `from` - Address sending XLM
    /// * `to` - Address receiving XLM
    /// * `amount` - Amount in stroops (must be positive)
    ///
    /// # Errors
    /// * `Error::NotInitialized` - contract not initialized
    /// * `Error::InvalidAmount` - amount is zero or negative
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) -> Result<(), Error> {
        // Check initialized
        let native_token_address: Address = env
            .storage()
            .instance()
            .get(&"native_token")
            .ok_or(Error::NotInitialized)?;

        // Validate amount
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        // Require authorization from sender
        from.require_auth();

        // Execute transfer via native token client
        let native_token = soroban_sdk::token::Client::new(&env, &native_token_address);
        native_token.transfer(&from, &to, &amount);

        // Emit event
        events::emit_native_transfer_executed(&env, from, to, amount);

        Ok(())
    }
}
