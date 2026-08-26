use soroban_sdk::{contracttype, Address, Env, Symbol};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    SuperAdmin,
    Role(Symbol, Address),
}

const INSTANCE_TTL_THRESHOLD: u32 = 100;
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400; // ~30 days

pub fn extend_instance_ttl(env: &Env) {
    let ledgers_to_live = INSTANCE_TTL_EXTEND_TO;
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, ledgers_to_live);
}

pub fn set_super_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::SuperAdmin, admin);
}

pub fn get_super_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::SuperAdmin)
}

pub fn has_role(env: &Env, role: &Symbol, account: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Role(role.clone(), account.clone()))
}

pub fn set_role(env: &Env, role: &Symbol, account: &Address) {
    let key = DataKey::Role(role.clone(), account.clone());
    env.storage().persistent().set(&key, &true);
    env.storage().persistent().extend_ttl(&key, 100, 6_307_200); // ~1 year
}

pub fn remove_role(env: &Env, role: &Symbol, account: &Address) {
    let key = DataKey::Role(role.clone(), account.clone());
    env.storage().persistent().remove(&key);
}





