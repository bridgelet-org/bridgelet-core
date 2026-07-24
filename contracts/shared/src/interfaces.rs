//! Issue #43: shared contract interface traits.
//!
//! These traits move the interface definitions that previously lived only in
//! the README into real, type-checked Rust. Each contract implements the trait
//! matching the methods it already exposes, so the interface stays in sync with
//! the implementation at compile time. The error type is left as an associated
//! type so each contract can keep its own `contracterror` enum.

use soroban_sdk::{Address, BytesN, Env};

/// Interface exposed by the ephemeral account contract.
pub trait EphemeralAccountInterface {
    /// Contract-specific error type.
    type Error;

    /// Initialize the ephemeral account with its restrictions.
    fn initialize(
        env: Env,
        creator: Address,
        expiry_ledger: u32,
        recovery_address: Address,
        authorized_controller: Address,
        authorized_signer: BytesN<32>,
        admin: Address,
        base_reserve: i128,
    ) -> Result<(), Self::Error>;

    /// Record an inbound payment to this account.
    fn record_payment(env: Env, amount: i128, asset: Address) -> Result<(), Self::Error>;

    /// Sweep funds to `destination`. `auth_signature` is accepted but not
    /// cryptographically verified by this contract.
    fn sweep(env: Env, destination: Address, auth_signature: BytesN<64>) -> Result<(), Self::Error>;

    /// Gas-free sweep path used by the sweep controller's claim flow.
    fn sweep_claim(env: Env, destination: Address) -> Result<(), Self::Error>;

    /// Whether the account has passed its expiry ledger.
    fn is_expired(env: Env) -> bool;
}

/// Interface exposed by the sweep controller contract.
pub trait SweepControllerInterface {
    /// Contract-specific error type.
    type Error;

    /// Initialize the controller with its authorized signer.
    fn initialize(
        env: Env,
        creator: Address,
        authorized_signer: BytesN<32>,
        authorized_destination: Option<Address>,
    ) -> Result<(), Self::Error>;

    /// Execute a sweep from an ephemeral account to `destination`.
    fn execute_sweep(
        env: Env,
        ephemeral_account: Address,
        destination: Address,
        auth_signature: BytesN<64>,
    ) -> Result<(), Self::Error>;

    /// Claim funds to `recipient` using Soroban auth entries.
    fn claim(env: Env, recipient: Address, ephemeral_account: Address) -> Result<(), Self::Error>;
}
