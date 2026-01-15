#![no_std]

mod errors;
mod events;
mod storage;

use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env};

pub use errors::Error;
pub use events::{AccountCreated, AccountExpired, PaymentReceived, SweepExecuted};
pub use storage::{AccountStatus, DataKey};

#[contract]
pub struct EphemeralAccountContract;

#[contractimpl]
impl EphemeralAccountContract {
    /// Initialize the ephemeral account with restrictions
    ///
    /// # Arguments
    /// * `creator` - Address that created this account
    /// * `expiry_ledger` - Ledger number when account expires
    /// * `recovery_address` - Address to return funds if expired
    ///
    /// # Errors
    /// Returns Error::AlreadyInitialized if called more than once
    pub fn initialize(
        env: Env,
        creator: Address,
        expiry_ledger: u32,
        recovery_address: Address,
    ) -> Result<(), Error> {
        // Check if already initialized
        if storage::is_initialized(&env) {
            return Err(Error::AlreadyInitialized);
        }

        // Verify creator authorization
        creator.require_auth();

        // Validate expiry is in future
        let current_ledger = env.ledger().sequence();
        if expiry_ledger <= current_ledger {
            return Err(Error::InvalidExpiry);
        }

        // Store initialization data
        storage::set_initialized(&env, true);
        storage::set_creator(&env, &creator);
        storage::set_expiry_ledger(&env, expiry_ledger);
        storage::set_recovery_address(&env, &recovery_address);
        storage::set_status(&env, AccountStatus::Active);

        // Emit event
        events::emit_account_created(&env, creator, expiry_ledger);

        Ok(())
    }

    /// Record an inbound payment to this ephemeral account
    /// Only the first payment is accepted
    ///
    /// # Arguments
    /// * `amount` - Payment amount
    /// * `asset` - Asset address
    ///
    /// # Errors
    /// Returns Error::PaymentAlreadyReceived if payment already recorded
    pub fn record_payment(env: Env, amount: i128, asset: Address) -> Result<(), Error> {
        // Check initialized
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }

        // Check if payment already received
        if storage::has_payment_received(&env) {
            return Err(Error::PaymentAlreadyReceived);
        }

        // Validate amount
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        // Record payment
        storage::set_payment_received(&env, true);
        storage::set_payment_amount(&env, amount);
        storage::set_payment_asset(&env, &asset);
        storage::set_status(&env, AccountStatus::PaymentReceived);

        // Emit event
        events::emit_payment_received(&env, amount, asset);

        Ok(())
    }

    /// Execute sweep to destination wallet
    /// Transfers all funds to the specified destination
    ///
    /// # Arguments
    /// * `destination` - Recipient wallet address
    /// * `auth_signature` - Authorization signature from off-chain system
    ///
    /// # Errors
    /// Returns Error::Unauthorized if authorization fails
    /// Returns Error::AlreadySwept if sweep already executed
    pub fn sweep(env: Env, destination: Address, auth_signature: BytesN<64>) -> Result<(), Error> {
        // Check initialized
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }

        // Check not already swept
        if storage::get_status(&env) == AccountStatus::Swept {
            return Err(Error::AlreadySwept);
        }

        // Check payment received
        if !storage::has_payment_received(&env) {
            return Err(Error::NoPaymentReceived);
        }

        // Check not expired
        if Self::is_expired(env.clone()) {
            return Err(Error::AccountExpired);
        }

        // Verify authorization signature
        // Note: In production, implement proper signature verification
        // For MVP, we trust the SDK to only call with valid signatures
        Self::verify_sweep_authorization(&env, &destination, &auth_signature)?;

        // Get payment details
        let amount = storage::get_payment_amount(&env);
        let asset = storage::get_payment_asset(&env);

        // Update status before transfer to prevent reentrancy
        storage::set_status(&env, AccountStatus::Swept);
        storage::set_swept_to(&env, &destination);

        // Note: Actual token transfer happens in the SDK via Stellar SDK
        // This contract enforces the business logic and authorization
        // The SDK will call this function, get approval, then execute the transfer

        // Emit event
        events::emit_sweep_executed(&env, destination, amount, asset);

        Ok(())
    }

    /// Check if account has expired
    pub fn is_expired(env: Env) -> bool {
        if !storage::is_initialized(&env) {
            return false;
        }

        let expiry_ledger = storage::get_expiry_ledger(&env);
        let current_ledger = env.ledger().sequence();

        current_ledger >= expiry_ledger
    }

    /// Get current account status
    pub fn get_status(env: Env) -> AccountStatus {
        if !storage::is_initialized(&env) {
            return AccountStatus::Active;
        }

        storage::get_status(&env)
    }

    /// Expire the account and return funds to recovery address
    /// Can only be called after expiry ledger is reached
    ///
    /// # Errors
    /// Returns Error::NotExpired if called before expiry ledger
    pub fn expire(env: Env) -> Result<(), Error> {
        // Check initialized
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }

        // Check not already swept or expired
        let status = storage::get_status(&env);
        if status == AccountStatus::Swept || status == AccountStatus::Expired {
            return Err(Error::InvalidStatus);
        }

        // Check if expired
        if !Self::is_expired(env.clone()) {
            return Err(Error::NotExpired);
        }

        // Get recovery address
        let recovery_address = storage::get_recovery_address(&env);

        // Update status
        storage::set_status(&env, AccountStatus::Expired);
        storage::set_swept_to(&env, &recovery_address);

        // Get payment details if payment was received
        let amount = if storage::has_payment_received(&env) {
            storage::get_payment_amount(&env)
        } else {
            0
        };

        // Emit event
        events::emit_account_expired(&env, recovery_address, amount);

        Ok(())
    }

    /// Get account information
    pub fn get_info(env: Env) -> Result<AccountInfo, Error> {
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }

        Ok(AccountInfo {
            creator: storage::get_creator(&env),
            status: storage::get_status(&env),
            expiry_ledger: storage::get_expiry_ledger(&env),
            recovery_address: storage::get_recovery_address(&env),
            payment_received: storage::has_payment_received(&env),
            payment_amount: if storage::has_payment_received(&env) {
                Some(storage::get_payment_amount(&env))
            } else {
                None
            },
            swept_to: storage::get_swept_to(&env),
        })
    }

    // Private helper functions

    fn verify_sweep_authorization(
        _env: &Env,
        _destination: &Address,
        _signature: &BytesN<64>,
    ) -> Result<(), Error> {
        // TODO: Implement proper signature verification
        // For MVP, we rely on off-chain SDK to only call with valid auth
        // Future: Verify signature against authorized signer
        Ok(())
    }
}

/// Account information structure
#[derive(Clone)]
#[contracttype]
pub struct AccountInfo {
    pub creator: Address,
    pub status: AccountStatus,
    pub expiry_ledger: u32,
    pub recovery_address: Address,
    pub payment_received: bool,
    pub payment_amount: Option<i128>,
    pub swept_to: Option<Address>,
}
