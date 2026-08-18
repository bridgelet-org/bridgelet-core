use soroban_sdk::{symbol_short, Address, Env};

pub fn emit_initialized(env: &Env, admin: Address) {
    env.events().publish((symbol_short!("init"),), admin);
}

pub fn emit_registered(env: &Env, account: Address, expiry_ledger: u32) {
    env.events()
        .publish((symbol_short!("register"),), (account, expiry_ledger));
}

pub fn emit_rescheduled(env: &Env, account: Address, expiry_ledger: u32) {
    env.events()
        .publish((symbol_short!("resched"),), (account, expiry_ledger));
}

pub fn emit_deregistered(env: &Env, account: Address) {
    env.events().publish((symbol_short!("dereg"),), account);
}
