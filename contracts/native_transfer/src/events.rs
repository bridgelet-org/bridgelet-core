use soroban_sdk::{contracttype, symbol_short, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeTransferExecuted {
    pub from: Address,
    pub to: Address,
    pub amount: i128,
}

pub fn emit_native_transfer_executed(env: &Env, from: Address, to: Address, amount: i128) {
    let event = NativeTransferExecuted { from, to, amount };
    env.events().publish((symbol_short!("native_tx"),), event);
}
