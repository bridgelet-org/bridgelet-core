use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub struct Sponsorship {
    pub sponsor: Address,
    pub cap: i128,
    pub drawn: i128,
}

#[contracttype]
pub enum DataKey {
    Initialized,
    SponsorBalance(Address),
    Sponsorship(Address),
}

// Initialization
pub fn is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Initialized)
}

pub fn set_initialized(env: &Env, value: bool) {
    env.storage().instance().set(&DataKey::Initialized, &value);
}

// Sponsor balance tracking
pub fn get_sponsor_balance(env: &Env, sponsor: &Address) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::SponsorBalance(sponsor.clone()))
        .unwrap_or(0)
}

pub fn set_sponsor_balance(env: &Env, sponsor: &Address, balance: i128) {
    env.storage()
        .instance()
        .set(&DataKey::SponsorBalance(sponsor.clone()), &balance);
}

pub fn add_sponsor_balance(env: &Env, sponsor: &Address, amount: i128) {
    let current = get_sponsor_balance(env, sponsor);
    set_sponsor_balance(env, sponsor, current + amount);
}

pub fn subtract_sponsor_balance(env: &Env, sponsor: &Address, amount: i128) {
    let current = get_sponsor_balance(env, sponsor);
    set_sponsor_balance(env, sponsor, current - amount);
}

// Sponsorship tracking
pub fn get_sponsorship(env: &Env, account: &Address) -> Option<Sponsorship> {
    env.storage()
        .instance()
        .get(&DataKey::Sponsorship(account.clone()))
}

pub fn set_sponsorship(env: &Env,account: &Address, sponsorship: &Sponsorship) {
    env.storage()
        .instance()
        .set(&DataKey::Sponsorship(account.clone()), sponsorship);
}

pub fn has_sponsorship(env: &Env, account: &Address) -> bool {
    env.storage()
        .instance()
        .has(&DataKey::Sponsorship(account.clone()))
}

// TTL management
const INSTANCE_TTL_THRESHOLD: u32 = 100;
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;

pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}
