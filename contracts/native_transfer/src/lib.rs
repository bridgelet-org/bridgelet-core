#![no_std]

mod errors;
mod events;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct NativeTransferContract;

#[contractimpl]
impl NativeTransferContract {}
