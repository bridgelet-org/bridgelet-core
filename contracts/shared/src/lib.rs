#![no_std]

mod interfaces;
mod types;

// Re-export the interface traits at the crate root so consumers can import
// them as `bridgelet_shared::EphemeralAccountInterface` /
// `bridgelet_shared::SweepControllerInterface` instead of having to know
// about the private `interfaces` submodule. Required by
// `contracts/ephemeral_account/src/lib.rs:11` and the planned
// `contracts/sweep_controller` interface implementation.
pub use interfaces::{EphemeralAccountInterface, SweepControllerInterface};

pub use types::{AccountInfo, AccountInitRequest, AccountInitResult, AccountStatus, Payment};
