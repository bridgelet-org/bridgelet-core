#![no_std]
use soroban_sdk::{contract, contractimpl, Env, symbol_short};

#[contract]
pub struct BridgeletCoreV2;

#[contractimpl]
impl BridgeletCoreV2 {
    pub fn v2_exclusive_feature(_env: Env) -> u32 {
        100
    }

    pub fn get_base_reserve(env: Env) -> u32 {
        env.storage().instance().get(&symbol_short!("Reserve")).unwrap_or(0)
    }
}