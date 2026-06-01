use soroban_sdk::{contracttype, symbol_short, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountDeployed {
    pub account_address: Address,
    pub creator: Address,
    pub expiry_ledger: u32,
}

pub fn emit_account_deployed(
    env: &Env,
    account_address: Address,
    creator: Address,
    expiry_ledger: u32,
) {
    let event = AccountDeployed {
        account_address,
        creator,
        expiry_ledger,
    };
    env.events().publish((symbol_short!("deployed"),), event);
}
