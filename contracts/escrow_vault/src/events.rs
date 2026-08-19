use soroban_sdk::{symbol_short, Address, Env};

pub fn emit_opened(
    env: &Env,
    id: u64,
    depositor: Address,
    beneficiary: Address,
    asset: Address,
    amount: i128,
    release_ledger: u32,
) {
    let topics = (symbol_short!("opened"), id, depositor);
    env.events()
        .publish(topics, (beneficiary, asset, amount, release_ledger));
}

pub fn emit_released(env: &Env, id: u64) {
    let topics = (symbol_short!("released"), id);
    env.events().publish(topics, ());
}

pub fn emit_canceled(env: &Env, id: u64) {
    let topics = (symbol_short!("canceled"), id);
    env.events().publish(topics, ());
}
