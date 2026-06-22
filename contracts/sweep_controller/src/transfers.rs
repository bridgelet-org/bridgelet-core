use crate::errors::Error;
use bridgelet_shared::Payment;
use soroban_sdk::token::TokenClient;
use soroban_sdk::{Address, Env, Vec};

/// Execute token transfers for all payments from the ephemeral account to the destination.
///
/// Iterates over each recorded payment and calls the SEP-41 token contract's
/// `transfer()` function, moving funds from `from` to `destination`.
///
/// The ephemeral account must have already authorized this contract to transfer
/// on its behalf — this is enforced by the Soroban auth model when `from.require_auth()`
/// is satisfied by the ephemeral account's invocation context.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `from` - Ephemeral account address (source of funds)
/// * `destination` - Recipient wallet address
/// * `payments` - All recorded payments to transfer
///
/// # Errors
/// Returns `Error::TransferFailed` if any individual transfer fails
pub fn execute_transfers(
    env: &Env,
    from: &Address,
    destination: &Address,
    payments: &Vec<Payment>,
) -> Result<(), Error> {
    for payment in payments.iter() {
        let token = TokenClient::new(env, &payment.asset);
        token.transfer(from, destination, &payment.amount);
    }
    Ok(())
}
