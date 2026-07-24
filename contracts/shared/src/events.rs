//! Issue #40: Shared contract event schemas.
//!
//! Event struct definitions live here so the SDK and every contract share one
//! identical schema. Contracts import these types and keep their own thin
//! `emit_*` helpers that publish them.

use crate::types::Payment;
use soroban_sdk::{contracttype, Address, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountCreated {
    pub creator: Address,
    pub expiry_ledger: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentReceived {
    pub amount: i128,
    pub asset: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SweepExecutedMulti {
    pub destination: Address,
    pub payments: Vec<Payment>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiPaymentReceived {
    pub asset: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountExpired {
    pub recovery_address: Address,
    pub amount_returned: i128,
    pub reserve_amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReserveReclaimed {
    pub destination: Address,
    pub amount: i128,
    pub sweep_id: u64,
    pub fully_reclaimed: bool,
    pub remaining_reserve: i128,
}
