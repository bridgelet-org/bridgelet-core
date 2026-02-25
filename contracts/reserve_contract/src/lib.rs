#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env};

pub use errors::Error;
pub use events::{BaseReserveUpdated, ContractInitialized};
pub use storage::DataKey;

/// Maximum allowed base reserve: 10 000 XLM = 100_000_000_000 stroops.
///
/// This ceiling exists to catch operator mistakes (e.g. passing a value in
/// XLM instead of stroops).  It can be raised if the Stellar network ever
/// increases its base reserve beyond this threshold.
const MAX_RESERVE_STROOPS: i128 = 100_000_000_000;

/// A focused on-chain contract that stores and exposes the base reserve
/// configuration for the Bridgelet system.
///
/// ## What is "base reserve"?
///
/// In the Stellar network every account must keep a minimum XLM balance
/// (the *base reserve*) to remain open.  Bridgelet's ephemeral accounts
/// need to know this amount so they can track how much XLM belongs to
/// user payments versus how much is network overhead that must be returned
/// to the creator when the account is closed.
///
/// This contract answers one question: **"what is the configured base
/// reserve, in stroops?"**
///
/// One XLM = 10,000,000 stroops.  Storing the value as an integer number
/// of stroops avoids floating-point arithmetic inside the contract.
///
/// ## Access control
///
/// The contract must be initialized once via [`initialize`] which stores
/// the admin address.  Only that admin may call [`set_base_reserve`].
#[contract]
pub struct ReserveContract;

#[contractimpl]
impl ReserveContract {
    /// One-time initialization that sets the admin address.
    ///
    /// Must be called exactly once before any other state-changing
    /// operation.  The `admin` address will be persisted and required
    /// to authorize every future [`set_base_reserve`] call.
    ///
    /// # Errors
    /// * [`Error::AlreadyInitialized`] – called more than once.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();

        storage::set_admin(&env, &admin);
        events::emit_initialized(&env, admin);

        Ok(())
    }

    /// Store a new base reserve amount (in stroops).
    ///
    /// Only the admin set during [`initialize`] may call this function.
    /// Each call overwrites the previous value and emits a
    /// `BaseReserveUpdated` event for off-chain auditability.
    ///
    /// # Arguments
    /// * `amount` – Base reserve expressed in stroops.  Must satisfy
    ///              `0 < amount <= MAX_RESERVE_STROOPS` (currently
    ///              100 000 000 000, i.e. 10 000 XLM).
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized.
    /// * [`Error::Unauthorized`]   – caller is not the admin.
    /// * [`Error::InvalidAmount`]  – `amount` is zero or negative.
    /// * [`Error::AmountTooLarge`] – `amount` exceeds the safety ceiling.
    ///
    /// # Example
    /// ```ignore
    /// // 100 XLM = 1_000_000_000 stroops
    /// client.set_base_reserve(&1_000_000_000i128);
    /// ```
    pub fn set_base_reserve(env: Env, amount: i128) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        // 1. Contract must be initialized
        let admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;

        // 2. Caller must be the admin
        admin.require_auth();

        // 3. Amount validation
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if amount > MAX_RESERVE_STROOPS {
            return Err(Error::AmountTooLarge);
        }

        // ── 4. Persist & emit
        let old_value = storage::get_base_reserve(&env).unwrap_or(0);
        storage::set_base_reserve(&env, amount);
        events::emit_base_reserve_updated(&env, old_value, amount, admin);

        Ok(())
    }

    /// Return the current base reserve amount (in stroops), if configured.
    ///
    /// # Returns
    /// * `Some(amount)` – the value previously set via [`set_base_reserve`].
    /// * `None`         – no base reserve has been stored yet.
    ///
    /// This safe default means consumers **must** handle the unset case
    /// explicitly, preventing silent use of a zero or garbage value.
    pub fn get_base_reserve(env: Env) -> Option<i128> {
        storage::extend_instance_ttl(&env);
        storage::get_base_reserve(&env)
    }

    /// Return the current base reserve amount (in stroops), or an error if
    /// it has not been configured yet.
    ///
    /// Use this variant when the caller requires the reserve to be set
    /// before proceeding (e.g. during a sweep flow that reads the reserve).
    ///
    /// # Errors
    /// Returns [`Error::ReserveNotSet`] when no value has been stored.
    pub fn require_base_reserve(env: Env) -> Result<i128, Error> {
        storage::extend_instance_ttl(&env);
        storage::get_base_reserve(&env).ok_or(Error::ReserveNotSet)
    }

    /// Returns `true` if a base reserve has been stored, `false` otherwise.
    ///
    /// Cheaper than calling [`get_base_reserve`] when only the presence of
    /// the key matters.
    pub fn has_base_reserve(env: Env) -> bool {
        storage::extend_instance_ttl(&env);
        storage::has_base_reserve(&env)
    }

    /// Returns the admin address, if the contract has been initialized.
    pub fn get_admin(env: Env) -> Option<Address> {
        storage::extend_instance_ttl(&env);
        storage::get_admin(&env)
    }
}
