// Issue #40: event schemas are defined once in the shared crate and re-exported
// here so existing `events::AccountCreated` (etc.) references keep working.
pub use bridgelet_shared::{
    AccountCreated, AccountExpired, MultiPaymentReceived, PaymentReceived, ReserveReclaimed,
    SweepExecutedMulti,
};

use bridgelet_shared::Payment;
use soroban_sdk::{symbol_short, Address, Env, Vec};

pub fn emit_account_created(env: &Env, creator: Address, expiry_ledger: u32) {
    let event = AccountCreated {
        creator,
        expiry_ledger,
    };
    env.events().publish((symbol_short!("created"),), event);
}

pub fn emit_payment_received(env: &Env, amount: i128, asset: Address) {
    let event = PaymentReceived { amount, asset };
    env.events().publish((symbol_short!("payment"),), event);
}

pub fn emit_sweep_executed_multi(env: &Env, destination: Address, payments: &Vec<Payment>) {
    let event = SweepExecutedMulti {
        destination,
        payments: payments.clone(),
    };
    env.events().publish((symbol_short!("swept_mul"),), event);
}

pub fn emit_multi_payment_received(env: &Env, asset: Address, amount: i128) {
    let event = MultiPaymentReceived { asset, amount };
    env.events().publish((symbol_short!("multi_pay"),), event);
}

pub fn emit_account_expired(
    env: &Env,
    recovery_address: Address,
    amount_returned: i128,
    reserve_amount: i128,
) {
    let event = AccountExpired {
        recovery_address,
        amount_returned,
        reserve_amount,
    };
    env.events().publish((symbol_short!("expired"),), event);
}

pub fn emit_reserve_reclaimed(
    env: &Env,
    destination: Address,
    amount: i128,
    sweep_id: u64,
    fully_reclaimed: bool,
    remaining_reserve: i128,
) {
    let event = ReserveReclaimed {
        destination,
        amount,
        sweep_id,
        fully_reclaimed,
        remaining_reserve,
    };
    env.events().publish((symbol_short!("reserve"),), event);
}
