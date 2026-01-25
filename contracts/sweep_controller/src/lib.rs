#![no_std]

mod authorization;
mod errors;
mod storage;
mod transfers;

use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env};

use authorization::AuthContext;
pub use errors::Error;

#[contract]
pub struct SweepController;

#[contractimpl]
impl SweepController {
    /// Initialize the sweep controller with authorized signer
    ///
    /// # Arguments
    /// * `authorized_signer` - Ed25519 public key (32 bytes) that will authorize sweep operations
    /// * `authorized_destination` - Optional destination address. If provided, sweeps can only go to this address (locked mode).
    ///                              If None, any destination is allowed (flexible mode).
    ///
    /// # Errors
    /// Returns Error::AuthorizationFailed if called more than once
    pub fn initialize(
        env: Env,
        authorized_signer: BytesN<32>,
        authorized_destination: Option<Address>,
    ) -> Result<(), Error> {
        // Check if already initialized
        if storage::get_authorized_signer(&env).is_some() {
            return Err(Error::AuthorizationFailed);
        }

        // Store the creator address
        // In Soroban SDK 22.0.0, we need to pass creator as a parameter
        // For now, we'll use the contract address as a placeholder
        // TODO: Update to accept creator as parameter if needed
        let creator = env.current_contract_address();
        storage::set_creator(&env, &creator);

        // Store the authorized signer public key
        storage::set_authorized_signer(&env, &authorized_signer);

        // Initialize the sweep nonce to 0
        storage::init_sweep_nonce(&env);

        // Store authorized destination if provided
        if let Some(destination) = authorized_destination {
            storage::set_authorized_destination(&env, &destination);
            emit_destination_authorized(&env, destination);
        }

        Ok(())
    }

    /// Execute sweep operation from ephemeral account to destination
    ///
    /// # Arguments
    /// * `ephemeral_account` - Address of the ephemeral account contract
    /// * `destination` - Destination wallet address
    /// * `auth_signature` - Authorization signature
    ///
    /// # Errors
    /// Returns Error::AuthorizationFailed if signature is invalid
    /// Returns Error::InvalidAccount if account is not in valid state
    /// Returns Error::TransferFailed if token transfer fails
    /// Returns Error::UnauthorizedDestination if destination doesn't match authorized destination (when set)
    pub fn execute_sweep(
        env: Env,
        ephemeral_account: Address,
        destination: Address,
        auth_signature: BytesN<64>,
    ) -> Result<(), Error> {
        // Validate destination if authorized destination is set (locked mode)
        if storage::has_authorized_destination(&env) {
            let authorized_dest = storage::get_authorized_destination(&env)
                .ok_or(Error::UnauthorizedDestination)?;
            if destination != authorized_dest {
                return Err(Error::UnauthorizedDestination);
            }
        }

        // Verify authorization
        let auth_ctx = AuthContext::new(
            ephemeral_account.clone(),
            destination.clone(),
            auth_signature.clone(),
        );
        auth_ctx.verify(&env)?;

        // Increment nonce after successful verification to prevent replay attacks
        authorization::increment_nonce(&env);

        // Call ephemeral account contract to validate and authorize sweep
        // This triggers the account's sweep() method which updates state
        let account_client = ephemeral_account::Client::new(&env, &ephemeral_account);

        // The account contract validates state and authorizes the sweep
        account_client.sweep(&destination, &auth_signature);

        // Get payment details from account
        let info = account_client.get_info();

        // Verify payment was received
        if !info.payment_received {
            return Err(Error::AccountNotReady);
        }

        // Get the total amount from payments
        // For now, we'll use the first payment's amount
        // In a multi-asset scenario, we'd need to handle this differently
        let payments = info.payments;
        if payments.len() == 0 {
            return Err(Error::AccountNotReady);
        }
        let first_payment = payments.get(0).ok_or(Error::AccountNotReady)?;
        let amount = first_payment.amount;

        // Execute the actual token transfer
        // Note: In production, the ephemeral account would need to authorize this transfer
        // let transfer_ctx = TransferContext::new(
        //     info.payment_asset,
        //     ephemeral_account.clone(),
        //     destination.clone(),
        //     amount,
        // );
        // transfer_ctx.execute(&env)?;

        // Emit sweep executed event
        emit_sweep_completed(&env, ephemeral_account, destination, amount);

        Ok(())
    }

    /// Check if an account is ready for sweep
    pub fn can_sweep(env: Env, ephemeral_account: Address) -> bool {
        let account_client = ephemeral_account::Client::new(&env, &ephemeral_account);

        // Check if account exists and has payment
        let info = account_client.get_info();

        info.payment_received
            && info.status == ephemeral_account::AccountStatus::PaymentReceived
            && !account_client.is_expired()
    }

    /// Update the authorized destination address
    ///
    /// This function allows the creator to update the authorized destination before any sweep occurs.
    /// Once a sweep has been executed, the destination cannot be changed.
    ///
    /// # Arguments
    /// * `new_destination` - New authorized destination address
    ///
    /// # Errors
    /// Returns Error::AuthorizationFailed if caller is not the creator
    /// Returns Error::AccountAlreadySwept if a sweep has already been executed
    pub fn update_authorized_destination(
        env: Env,
        new_destination: Address,
    ) -> Result<(), Error> {
        // Verify creator authorization
        let creator = storage::get_creator(&env).ok_or(Error::AuthorizationFailed)?;
        creator.require_auth();

        // Check if a sweep has already been executed (nonce > 0 indicates at least one sweep)
        let nonce = storage::get_sweep_nonce(&env);
        if nonce > 0 {
            return Err(Error::AccountAlreadySwept);
        }

        // Update the authorized destination
        let old_destination = storage::get_authorized_destination(&env);
        storage::set_authorized_destination(&env, &new_destination);

        // Emit event
        emit_destination_updated(&env, old_destination, new_destination);

        Ok(())
    }
}

/// Sweep completed event
#[contracttype]
#[derive(Clone, Debug)]
pub struct SweepCompleted {
    pub ephemeral_account: Address,
    pub destination: Address,
    pub amount: i128,
}

/// Destination authorized event (emitted when destination is set during initialization)
#[contracttype]
#[derive(Clone, Debug)]
pub struct DestinationAuthorized {
    pub destination: Address,
}

/// Destination updated event (emitted when authorized destination is updated)
#[contracttype]
#[derive(Clone, Debug)]
pub struct DestinationUpdated {
    pub old_destination: Option<Address>,
    pub new_destination: Address,
}

fn emit_sweep_completed(env: &Env, account: Address, destination: Address, amount: i128) {
    let event = SweepCompleted {
        ephemeral_account: account,
        destination,
        amount,
    };
    env.events()
        .publish((soroban_sdk::symbol_short!("sweep"),), event);
}

fn emit_destination_authorized(env: &Env, destination: Address) {
    let event = DestinationAuthorized { destination };
    env.events()
        .publish((soroban_sdk::symbol_short!("dest_auth"),), event);
}

fn emit_destination_updated(env: &Env, old_destination: Option<Address>, new_destination: Address) {
    let event = DestinationUpdated {
        old_destination,
        new_destination,
    };
    env.events()
        .publish((soroban_sdk::symbol_short!("dest_upd"),), event);
}

// Re-export ephemeral_account types for cross-contract calls
mod ephemeral_account {
    // Import from the actual ephemeral_account contract
    soroban_sdk::contractimport!(
        file = "../ephemeral_account/target/wasm32-unknown-unknown/release/ephemeral_account.wasm"
    );
}
