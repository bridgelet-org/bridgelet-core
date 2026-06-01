use soroban_sdk::{contracttype, Address, BytesN, Env, Map, Vec};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    EphemeralWasmHash,
    DeployedAccounts,
    AllAccounts,
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

pub fn set_ephemeral_wasm_hash(env: &Env, hash: &BytesN<32>) {
    env.storage()
        .instance()
        .set(&DataKey::EphemeralWasmHash, hash);
}

pub fn get_ephemeral_wasm_hash(env: &Env) -> Option<BytesN<32>> {
    env.storage().instance().get(&DataKey::EphemeralWasmHash)
}

pub fn add_deployed_account(env: &Env, creator: &Address, account: &Address) {
    let mut by_creator: Map<Address, Vec<Address>> = env
        .storage()
        .instance()
        .get(&DataKey::DeployedAccounts)
        .unwrap_or_else(|| Map::new(env));

    let mut creator_accounts = by_creator
        .get(creator.clone())
        .unwrap_or_else(|| Vec::new(env));
    creator_accounts.push_back(account.clone());
    by_creator.set(creator.clone(), creator_accounts);

    env.storage()
        .instance()
        .set(&DataKey::DeployedAccounts, &by_creator);

    let mut all_accounts: Vec<Address> = env
        .storage()
        .instance()
        .get(&DataKey::AllAccounts)
        .unwrap_or_else(|| Vec::new(env));
    all_accounts.push_back(account.clone());
    env.storage()
        .instance()
        .set(&DataKey::AllAccounts, &all_accounts);
}

pub fn get_accounts_by_creator(env: &Env, creator: &Address) -> Vec<Address> {
    let by_creator: Map<Address, Vec<Address>> = env
        .storage()
        .instance()
        .get(&DataKey::DeployedAccounts)
        .unwrap_or_else(|| Map::new(env));

    by_creator
        .get(creator.clone())
        .unwrap_or_else(|| Vec::new(env))
}

pub fn get_all_accounts(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::AllAccounts)
        .unwrap_or_else(|| Vec::new(env))
}
