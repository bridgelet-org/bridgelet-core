use soroban_sdk::{contracttype, Address, Env, Vec};

#[derive(Clone)]
#[contracttype]
pub struct ScheduleEntry {
    pub account: Address,
    pub expiry_ledger: u32,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Entries,
}

const INSTANCE_TTL_THRESHOLD: u32 = 100;
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;
const PERSISTENT_TTL_THRESHOLD: u32 = 100;
const PERSISTENT_TTL_EXTEND_TO: u32 = 6_307_200;

pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn get_entries(env: &Env) -> Vec<ScheduleEntry> {
    env.storage()
        .persistent()
        .get(&DataKey::Entries)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_entries(env: &Env, entries: &Vec<ScheduleEntry>) {
    env.storage().persistent().set(&DataKey::Entries, entries);
    env.storage().persistent().extend_ttl(
        &DataKey::Entries,
        PERSISTENT_TTL_THRESHOLD,
        PERSISTENT_TTL_EXTEND_TO,
    );
}
