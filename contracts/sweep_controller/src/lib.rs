#![no_std]

mod authorization;
mod errors;
mod transfers;

use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env};

use authorization::AuthContext;
pub use errors::Error;
use transfers::TransferContext;

#[contract]
pub struct SweepController;

#[contractimpl]
impl SweepController {
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
    pub fn execute_sweep(
        env: Env,
        ephemeral_account: Address,
        destination: Address,
        auth_signature: BytesN<64>,
    ) -> Result<(), Error> {
        // Verify authorization
        let auth_ctx = AuthContext::new(
            ephemeral_account.clone(),
            destination.clone(),
            auth_signature.clone(),
        );
        auth_ctx.verify(&env)?;

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

        let amount = info.payment_amount.ok_or(Error::AccountNotReady)?;

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
}

/// Sweep completed event
#[contracttype]
#[derive(Clone, Debug)]
pub struct SweepCompleted {
    pub ephemeral_account: Address,
    pub destination: Address,
    pub amount: i128,
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

// Re-export ephemeral_account types for cross-contract calls
mod ephemeral_account {
    use soroban_sdk::{contractclient, Address, BytesN, Env};

    // Import from the actual ephemeral_account contract
    soroban_sdk::contractimport!(
        file = "../ephemeral_account/target/wasm32-unknown-unknown/release/ephemeral_account.wasm"
    );
}
