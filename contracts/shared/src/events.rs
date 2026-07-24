use crate::types::Payment;
use soroban_sdk::{contracttype, Address, Vec};

/// Issue #40: contract event definitions live in the shared crate so the SDK
/// and every contract reference identical event schemas.

/// Emitted when an ephemeral account is created.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountCreated {
    pub creator: Address,
    pub expiry_ledger: u32,
}

/// Emitted when a single payment is received by an ephemeral account.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentReceived {
    pub amount: i128,
    pub asset: Address,
}

/// Emitted when an account is swept, carrying every recorded payment.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SweepExecutedMulti {
    pub destination: Address,
    pub payments: Vec<Payment>,
}

/// Emitted when a payment is received while multiple payments are tracked.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiPaymentReceived {
    pub asset: Address,
    pub amount: i128,
}

/// Emitted when an expired account returns funds to its recovery address.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountExpired {
    pub recovery_address: Address,
    pub amount_returned: i128,
    pub reserve_amount: i128,
}

/// Emitted when reserve funds are reclaimed after a sweep.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReserveReclaimed {
    pub destination: Address,
    pub amount: i128,
    pub sweep_id: u64,
    pub fully_reclaimed: bool,
    pub remaining_reserve: i128,
}
