#![no_std]

mod errors;
mod events;

use soroban_sdk::{contract, contractimpl, Env};

pub use errors::Error;
pub use events::NativeTransferExecuted;

#[contract]
pub struct NativeTransferContract;

#[contractimpl]
impl NativeTransferContract {
    // Implementation coming in a future issue
}
