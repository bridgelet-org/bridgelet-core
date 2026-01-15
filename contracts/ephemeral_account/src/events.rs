use soroban_sdk::{contracttype, symbol_short, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountCreated {
    pub creator: Address,
    pub expiry_ledger: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentReceived {
    pub amount: i128,
    pub asset: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SweepExecuted {
    pub destination: Address,
    pub amount: i128,
    pub asset: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountExpired {
    pub recovery_address: Address,
    pub amount_returned: i128,
}

pub fn emit_account_created(env: &Env, creator: Address, expiry_ledger: u32) {
    let event = AccountCreated {
        creator,
        expiry_ledger,
    };
    env.events().publish((symbol_short!("created"),), event);
}

pub fn emit_payment_received(env: &Env, amount: i128, asset: Address) {
    let event = PaymentReceived { amount, asset };
    env.events().publish((symbol_short!("payment"),), event);
}

pub fn emit_sweep_executed(env: &Env, destination: Address, amount: i128, asset: Address) {
    let event = SweepExecuted {
        destination,
        amount,
        asset,
    };
    env.events().publish((symbol_short!("swept"),), event);
}

pub fn emit_account_expired(env: &Env, recovery_address: Address, amount_returned: i128) {
    let event = AccountExpired {
        recovery_address,
        amount_returned,
    };
    env.events().publish((symbol_short!("expired"),), event);
}
