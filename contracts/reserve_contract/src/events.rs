use soroban_sdk::{contracttype, symbol_short, Address, Env};

// ─── Event payloads ─────────────────────────────────────────────────────────

/// Emitted once when [`ReserveContract::initialize`] is called successfully.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractInitialized {
    pub admin: Address,
}

/// Emitted every time [`ReserveContract::set_base_reserve`] stores a new value.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BaseReserveUpdated {
    pub old_value: i128,
    pub new_value: i128,
    pub admin: Address,
}

// ─── Emit helpers ───────────────────────────────────────────────────────────

/// Publish the `initialized` event.
pub fn emit_initialized(env: &Env, admin: Address) {
    let event = ContractInitialized { admin };
    env.events().publish((symbol_short!("init"),), event);
}

/// Publish the `reserve` event with old and new values for auditability.
///
/// `old_value` is `0` when no previous reserve existed.
pub fn emit_base_reserve_updated(env: &Env, old_value: i128, new_value: i128, admin: Address) {
    let event = BaseReserveUpdated {
        old_value,
        new_value,
        admin,
    };
    env.events().publish((symbol_short!("reserve"),), event);
}
