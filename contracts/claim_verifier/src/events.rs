use soroban_sdk::{contracttype, symbol_short, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationSucceeded {
    pub destination: Address,
    pub nonce: u64,
}

pub fn emit_verification_succeeded(env: &Env, destination: Address, nonce: u64) {
    let event = VerificationSucceeded { destination, nonce };
    env.events().publish((symbol_short!("verified"),), event);
}
