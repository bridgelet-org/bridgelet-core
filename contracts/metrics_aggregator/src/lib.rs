#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

pub use errors::Error;

/// The public interface of the metrics aggregator.
///
/// `increment`, `get` and `authorize_writer` are the core three; the remaining
/// methods are the bootstrap (`initialize`) and the read/revoke paths needed to
/// operate them.
pub trait MetricsAggregatorInterface {
    /// One-time initialization that sets the admin address.
    fn initialize(env: Env, admin: Address) -> Result<(), Error>;

    /// Add `amount` to the running total for `metric`.
    fn increment(env: Env, writer: Address, metric: Symbol, amount: i128) -> Result<(), Error>;

    /// Read the running total for `metric` (`0` if never incremented).
    fn get(env: Env, metric: Symbol) -> i128;

    /// Authorize `writer` to call `increment`.
    fn authorize_writer(env: Env, admin: Address, writer: Address) -> Result<(), Error>;

    /// Revoke a writer's authorization to call `increment`.
    fn revoke_writer(env: Env, admin: Address, writer: Address) -> Result<(), Error>;

    /// Return `true` if `writer` may currently call `increment`.
    fn is_writer(env: Env, writer: Address) -> bool;

    /// Return the admin address, if the contract has been initialized.
    fn get_admin(env: Env) -> Option<Address>;
}

/// A standalone counter contract that holds aggregate platform statistics
/// on-chain.
///
/// ## Purpose
///
/// Aggregate numbers — accounts created, sweeps executed, accounts expired,
/// total volume per asset — otherwise exist only as indexed events, which means
/// every consumer has to trust an indexer to have replayed the whole chain
/// correctly. This contract gives dashboards and monitoring tools a single
/// number they can read straight from the ledger instead.
///
/// It is deliberately standalone: no other contract in the workspace needs to
/// change for it to be deployed, and a contract that *wants* to report a metric
/// simply gets authorized as a writer and calls `increment`.
///
/// ## Interface
///
/// | Method             | Description                                     |
/// |--------------------|-------------------------------------------------|
/// | `initialize`       | One-time; sets the admin.                       |
/// | `authorize_writer` | Admin-only; lets `writer` call `increment`.     |
/// | `revoke_writer`    | Admin-only; withdraws an authorization.         |
/// | `increment`        | Writer-only; adds `amount` to a metric total.   |
/// | `get`              | Unrestricted read; `0` for an untouched metric. |
/// | `is_writer`        | Unrestricted read; authorization check.         |
/// | `get_admin`        | Unrestricted read; the configured admin.        |
///
/// ## Access control
///
/// * Only the `admin` set during `initialize` may authorize or revoke writers.
/// * Only an authorized writer may call `increment`, and the call must
///   additionally pass `require_auth` — being on the writer list is not enough
///   on its own, so a third party cannot inflate a metric by naming an
///   authorized writer it does not control.
/// * Reads (`get`, `is_writer`, `get_admin`) are unrestricted: these are public
///   platform statistics.
///
/// ## Counter semantics
///
/// Counters are **monotonic**: `increment` accepts only strictly positive
/// amounts, so a total can never be walked back and there is no decrement or
/// reset path. Metrics are independent — incrementing one never affects
/// another — and are identified by an arbitrary `Symbol`, so a new metric can be
/// introduced without redeploying. A metric that has never been written reads as
/// `0` rather than erroring, which keeps a dashboard's first call, made before
/// any activity has happened, from having to special-case a failure.
///
/// Each `increment` is an independent read-modify-write on a single ledger
/// entry. Soroban serializes conflicting writes to the same entry, so
/// concurrent submissions from different writers either both apply — in
/// whichever order the ledger settles them, reaching the same total either way —
/// or the losing transaction fails outright and can be retried. No update is
/// silently dropped.
#[contract]
pub struct MetricsAggregator;

#[contractimpl]
impl MetricsAggregatorInterface for MetricsAggregator {
    /// One-time initialization that sets the admin address.
    ///
    /// Must be called exactly once. The `admin` address is persisted and is
    /// required to authorize or revoke every writer thereafter.
    ///
    /// # Errors
    /// * [`Error::AlreadyInitialized`] – called more than once.
    fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();

        storage::set_admin(&env, &admin);
        events::emit_initialized(&env, admin);

        Ok(())
    }

    /// Add `amount` to the running total for `metric`.
    ///
    /// Repeated calls accumulate: three increments of 5 leave the metric at 15.
    /// Distinct metrics are independent counters.
    ///
    /// # Arguments
    /// * `writer` – must be an authorized writer and must authorize the call.
    /// * `metric` – the counter to add to; any `Symbol` is accepted.
    /// * `amount` – strictly positive amount to add.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`]     – contract has not been initialized.
    /// * [`Error::InvalidAmount`]      – `amount` is zero or negative.
    /// * [`Error::UnauthorizedWriter`] – `writer` is not authorized.
    /// * [`Error::CounterOverflow`]    – the total would exceed `i128::MAX`; the
    ///   stored value is left unchanged.
    fn increment(env: Env, writer: Address, metric: Symbol, amount: i128) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        if !storage::is_authorized_writer(&env, &writer) {
            return Err(Error::UnauthorizedWriter);
        }

        writer.require_auth();

        let current = storage::get_counter(&env, &metric);
        let total = current.checked_add(amount).ok_or(Error::CounterOverflow)?;

        storage::set_counter(&env, &metric, total);
        events::emit_metric_incremented(&env, writer, metric, amount, total);

        Ok(())
    }

    /// Return the running total for `metric`.
    ///
    /// A metric that has never been incremented returns `0`; this is not an
    /// error, and is indistinguishable from a metric whose total is genuinely
    /// zero.
    fn get(env: Env, metric: Symbol) -> i128 {
        storage::extend_instance_ttl(&env);
        storage::get_counter(&env, &metric)
    }

    /// Authorize `writer` to call [`MetricsAggregator::increment`].
    ///
    /// Only the admin may call this. Authorizing an already-authorized writer is
    /// a no-op.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized.
    /// * [`Error::Unauthorized`]   – `admin` does not match the stored admin.
    fn authorize_writer(env: Env, admin: Address, writer: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        storage::authorize_writer(&env, &writer);
        events::emit_writer_authorized(&env, writer, admin);

        Ok(())
    }

    /// Revoke a writer's authorization to call
    /// [`MetricsAggregator::increment`].
    ///
    /// Only the admin may call this. Revoking an address that was never
    /// authorized is a no-op. Totals the writer already recorded are unaffected
    /// — counters are append-only.
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – contract has not been initialized.
    /// * [`Error::Unauthorized`]   – `admin` does not match the stored admin.
    fn revoke_writer(env: Env, admin: Address, writer: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let stored_admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        admin.require_auth();

        storage::revoke_writer(&env, &writer);
        events::emit_writer_revoked(&env, writer, admin);

        Ok(())
    }

    /// Return `true` if `writer` is currently authorized to call `increment`.
    fn is_writer(env: Env, writer: Address) -> bool {
        storage::extend_instance_ttl(&env);
        storage::is_authorized_writer(&env, &writer)
    }

    /// Return the admin address, if the contract has been initialized.
    fn get_admin(env: Env) -> Option<Address> {
        storage::extend_instance_ttl(&env);
        storage::get_admin(&env)
    }
}
