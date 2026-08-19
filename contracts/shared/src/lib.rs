#![no_std]

pub mod errors;
pub mod passphrase;
pub mod storage_keys;
mod types;

#[cfg(test)]
pub mod test_utils;

mod events;
pub mod interfaces;

pub use events::{
    AccountCreated, AccountExpired, DepositEvent, DrawEvent, MultiPaymentReceived, PaymentReceived,
    ReserveReclaimed, SponsorshipCreatedEvent, SweepExecutedMulti,
};

pub use interfaces::{EphemeralAccountInterface, SweepControllerInterface};

pub use errors::SharedError;
pub use storage_keys::StorageKey;
pub use types::{
    AccountInfo, AccountInitRequest, AccountInitResult, AccountStatus, AssetBalance,
    ContractVersion, Payment, SweepPayload,
};
