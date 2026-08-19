use soroban_sdk::{contracttype, symbol_short, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DepositEvent {
    pub sponsor: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SponsorshipCreatedEvent {
    pub sponsor: Address,
    pub account: Address,
    pub cap: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrawEvent {
    pub account: Address,
    pub amount: i128,
    pub remaining: i128,
}

pub fn emit_deposit(env: &Env, sponsor: Address, amount: i128) {
    let event = DepositEvent { sponsor, amount };
    env.events().publish((symbol_short!("deposit"),), event);
}

pub fn emit_sponsorship_created(env: &Env, sponsor: Address, account: Address, cap: i128) {
    let event = SponsorshipCreatedEvent {
        sponsor,
        account,
        cap,
    };
    env.events()
        .publish((symbol_short!("sponsor"),), event);
}

pub fn emit_draw(env: &Env, account: Address, amount: i128, remaining: i128) {
    let event = DrawEvent {
        account,
        amount,
        remaining,
    };
    env.events().publish((symbol_short!("draw"),), event);
}
