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
/// Note: Soroban's SEP-41 `transfer()` traps on failure, so individual
/// transfer errors cannot be recovered — the entire transaction rolls back.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `from` - Ephemeral account address (source of funds)
/// * `destination` - Recipient wallet address
/// * `payments` - All recorded payments to transfer
pub fn execute_transfers(
    env: &Env,
    from: &Address,
    destination: &Address,
    payments: &Vec<Payment>,
) {
    for payment in payments.iter() {
        let token = TokenClient::new(env, &payment.asset);
        token.transfer(from, destination, &payment.amount);
    }
}
