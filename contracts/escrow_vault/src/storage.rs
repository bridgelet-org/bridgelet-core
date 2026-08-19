use soroban_sdk::{contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    NextId,
    Escrow(u64),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowEntry {
    pub depositor: Address,
    pub beneficiary: Address,
    pub asset: Address,
    pub amount: i128,
    pub release_ledger: u32,
}

const INSTANCE_TTL_THRESHOLD: u32 = 100_000;
const INSTANCE_TTL_EXTEND: u32 = 518_400;
const PERSISTENT_TTL_THRESHOLD: u32 = 100_000;
const PERSISTENT_TTL_EXTEND: u32 = 518_400;

pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
}

pub fn get_next_id(env: &Env) -> u64 {
    env.storage().instance().get(&DataKey::NextId).unwrap_or(1)
}

pub fn increment_next_id(env: &Env) -> u64 {
    let id = get_next_id(env);
    env.storage().instance().set(&DataKey::NextId, &(id + 1));
    id
}

pub fn get_escrow(env: &Env, id: u64) -> Option<EscrowEntry> {
    let key = DataKey::Escrow(id);
    if let Some(entry) = env.storage().persistent().get::<_, EscrowEntry>(&key) {
        env.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND,
        );
        Some(entry)
    } else {
        None
    }
}

pub fn set_escrow(env: &Env, id: u64, entry: &EscrowEntry) {
    let key = DataKey::Escrow(id);
    env.storage().persistent().set(&key, entry);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND);
}

pub fn remove_escrow(env: &Env, id: u64) {
    let key = DataKey::Escrow(id);
    env.storage().persistent().remove(&key);
}
