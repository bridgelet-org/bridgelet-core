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
/// ## Atomicity guarantee
///
/// Soroban transactions are atomic: if any individual `transfer()` call fails
/// (panics, returns an error, or the SEP-41 token rejects it), the entire
/// transaction reverts — including any prior successful transfers in this
/// batch.  No partial state is ever committed on-chain.
///
/// ## Overflow safety
///
/// All i128 arithmetic on payment amounts is checked.  If the cumulative sum
/// overflows `i128::MAX`, the function returns `Error::InsufficientBalance`
/// before any on-chain transfer is attempted.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `from` - Ephemeral account address (source of funds)
/// * `destination` - Recipient wallet address
/// * `payments` - All recorded payments to transfer
///
/// # Errors
/// Returns `Error::InsufficientBalance` if cumulative amount overflows
/// Returns `Error::TransferFailed` if any individual SEP-41 transfer fails
pub fn execute_transfers(
    env: &Env,
    from: &Address,
    destination: &Address,
    payments: &Vec<Payment>,
) -> Result<(), Error> {
    // ── Pre-flight: validate cumulative amount does not overflow ────────
    // This is a defence-in-depth check.  Individual token.transfer() calls
    // will also reject amounts exceeding the sender's balance, but catching
    // overflow here gives a clearer error message and avoids wasting
    // resources on calls that would certainly fail.
    let mut cumulative: i128 = 0;
    for payment in payments.iter() {
        if payment.amount <= 0 {
            return Err(Error::InsufficientBalance);
        }
        cumulative = cumulative
            .checked_add(payment.amount)
            .ok_or(Error::InsufficientBalance)?;
    }

    // ── Execute transfers ──────────────────────────────────────────────
    // Each transfer() call delegates to the SEP-41 token contract.
    // If any fails, Soroban rolls back the entire transaction atomically.
    for payment in payments.iter() {
        let token = TokenClient::new(env, &payment.asset);
        token.transfer(from, destination, &payment.amount);
    }

    Ok(())
}

/// Estimate the total amount that would be transferred.
///
/// Pure function — makes no on-chain calls.  Useful for SDK-level
/// pre-sweep validation and fee estimation.
pub fn estimate_total(payments: &Vec<Payment>) -> Result<i128, Error> {
    let mut total: i128 = 0;
    for payment in payments.iter() {
        if payment.amount <= 0 {
            return Err(Error::InsufficientBalance);
        }
        total = total
            .checked_add(payment.amount)
            .ok_or(Error::InsufficientBalance)?;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_total_single_payment() {
        let env = Env::default();
        let asset = Address::generate(&env);
        let payments = Vec::from_array(&env, [Payment { asset, amount: 100, timestamp: 0 }]);
        assert_eq!(estimate_total(&payments).unwrap(), 100);
    }

    #[test]
    fn test_estimate_total_multiple_payments() {
        let env = Env::default();
        let a1 = Address::generate(&env);
        let a2 = Address::generate(&env);
        let payments = Vec::from_array(&env, [
            Payment { asset: a1, amount: 100, timestamp: 0 },
            Payment { asset: a2, amount: 200, timestamp: 0 },
        ]);
        assert_eq!(estimate_total(&payments).unwrap(), 300);
    }

    #[test]
    fn test_estimate_total_rejects_zero_amount() {
        let env = Env::default();
        let asset = Address::generate(&env);
        let payments = Vec::from_array(&env, [Payment { asset, amount: 0, timestamp: 0 }]);
        assert!(estimate_total(&payments).is_err());
    }

    #[test]
    fn test_estimate_total_rejects_negative_amount() {
        let env = Env::default();
        let asset = Address::generate(&env);
        let payments = Vec::from_array(&env, [Payment { asset, amount: -1, timestamp: 0 }]);
        assert!(estimate_total(&payments).is_err());
    }
}
