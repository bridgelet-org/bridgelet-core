use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleGranted {
    pub role: Symbol,
    pub account: Address,
    pub grantee: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleRevoked {
    pub role: Symbol,
    pub account: Address,
    pub revoker: Address,
}

pub fn emit_role_granted(env: &Env, role: Symbol, account: Address, grantee: Address) {
    env.events().publish(
        (symbol_short!("role"), symbol_short!("granted")),
        RoleGranted {
            role,
            account,
            grantee,
        },
    );
}

pub fn emit_role_revoked(env: &Env, role: Symbol, account: Address, revoker: Address) {
    env.events().publish(
        (symbol_short!("role"), symbol_short!("revoked")),
        RoleRevoked {
            role,
            account,
            revoker,
        },
    );
}




