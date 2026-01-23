use crate::storage::Payment;
use soroban_sdk::{contracttype, symbol_short, Address, Env, Map, Vec};

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
pub struct SweepExecutedMulti {
    pub destination: Address,
    pub payments: Vec<Payment>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiPaymentReceived {
    pub asset: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountExpired {
    pub recovery_address: Address,
    pub amount_returned: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetAmount {
    pub asset: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SweepExecuted {
    pub destination: Address,
    pub assets: Vec<AssetAmount>,
    pub reserve_reclaimed: i128,  // New: track reclaimed reserve
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReserveReclaimed {
    pub recipient: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountExpired {
    pub recovery_address: Address,
    pub total_assets: u32,
    pub reserve_returned: i128,  // New: track returned reserve
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

pub fn emit_sweep_executed_multi(env: &Env, destination: Address, payments: &Vec<Payment>) {
    let event = SweepExecutedMulti {
        destination,
        payments: payments.clone(),
    };
    env.events().publish((symbol_short!("swept_mul"),), event);
}

pub fn emit_multi_payment_received(env: &Env, asset: Address, amount: i128) {
    let event = MultiPaymentReceived { asset, amount };
    env.events().publish((symbol_short!("multi_pay"),), event);
}

pub fn emit_account_expired(env: &Env, recovery_address: Address, amount_returned: i128) {
    let event = AccountExpired {
        recovery_address,
        amount_returned,
    };
    env.events().publish((symbol_short!("expired"),), event);
}

pub fn emit_sweep_executed(env: &Env, destination: Address, payments: &Map<Address, i128>, reserve_amount: i128) {
    let mut assets = Vec::new(env);
    
    for key in payments.keys() {
        let asset = key;
        let amount = payments.get(asset.clone()).unwrap();
        assets.push_back(AssetAmount { asset, amount });
    }
    
    let event = SweepExecuted {
        destination,
        assets,
        reserve_reclaimed: reserve_amount,
    };
    env.events().publish((symbol_short!("swept"),), event);
}

pub fn emit_reserve_reclaimed(env: &Env, recipient: Address, amount: i128) {
    let event = ReserveReclaimed { recipient, amount };
    env.events()
        .publish((symbol_short!("reserve"),), event);
}

pub fn emit_account_expired(env: &Env, recovery_address: Address, total_assets: u32, reserve_amount: i128) {
    let event = AccountExpired {
        recovery_address,
        total_assets,
        reserve_returned: reserve_amount,
    };
    env.events().publish((symbol_short!("expired"),), event);
}
