use soroban_sdk::{contracttype, Address, Env, Symbol};

/// Storage keys used by the metrics aggregator.
///
/// `Admin` lives in **instance** storage: it is read by every authorization
/// check and is refreshed on every entry-point call, so it never risks
/// archival while the contract is in use.
///
/// `Writer(address)` and `Counter(metric)` live in **persistent** storage.
/// Counters are the contract's product — a dashboard reading a total months
/// after the last write must still see it — and a writer authorization that
/// silently lapsed would break the writers that depend on it. Both are given
/// a ~1-year TTL that is refreshed on access.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Admin address. Set once during `initialize`.
    Admin,

    /// Marker that `writer` is authorized to call `increment`.
    ///
    /// Key present → authorized. Key absent → not authorized.
    /// The value stored is a `bool` (always `true`) used as a sentinel.
    Writer(Address),

    /// The running total for `metric`.
    ///
    /// Key absent → the metric has never been incremented, which reads as `0`
    /// rather than as an error.
    Counter(Symbol),
}

// ── TTL constants ────────────────────────────────────────────────────────────

/// Minimum remaining ledgers before we extend the instance TTL.
const INSTANCE_TTL_THRESHOLD: u32 = 100;

/// Target ledger lifetime for instance storage (~30 days at ~5 s/ledger).
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;

/// Minimum remaining ledgers before we extend a persistent entry's TTL.
const PERSISTENT_TTL_THRESHOLD: u32 = 100;

/// Target ledger lifetime for persistent entries (~1 year at ~5 s/ledger).
const PERSISTENT_TTL_EXTEND_TO: u32 = 6_307_200; // ≈ 1 year

// ── Instance TTL ─────────────────────────────────────────────────────────────

/// Extend the instance storage TTL. Called from every public entry-point.
pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

// ── Admin helpers ────────────────────────────────────────────────────────────

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Admin)
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

// ── Writer helpers ───────────────────────────────────────────────────────────

/// Mark `writer` as authorized to call `increment`.
pub fn authorize_writer(env: &Env, writer: &Address) {
    let key = DataKey::Writer(writer.clone());
    env.storage().persistent().set(&key, &true);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

/// Remove `writer`'s authorization. A no-op if it was never authorized.
pub fn revoke_writer(env: &Env, writer: &Address) {
    env.storage()
        .persistent()
        .remove(&DataKey::Writer(writer.clone()));
}

/// Return `true` if `writer` is currently authorized to call `increment`.
pub fn is_authorized_writer(env: &Env, writer: &Address) -> bool {
    let key = DataKey::Writer(writer.clone());
    if !env.storage().persistent().has(&key) {
        return false;
    }
    // Reading an authorization is proof it is still in use — keep it alive.
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
    true
}

// ── Counter helpers ──────────────────────────────────────────────────────────

/// Return the running total for `metric`, or `0` if it has never been
/// incremented.
///
/// A metric that has never been written is indistinguishable from one whose
/// total is genuinely zero, and both correctly read as `0`.
pub fn get_counter(env: &Env, metric: &Symbol) -> i128 {
    let key = DataKey::Counter(metric.clone());
    match env.storage().persistent().get::<_, i128>(&key) {
        Some(value) => {
            env.storage().persistent().extend_ttl(
                &key,
                PERSISTENT_TTL_THRESHOLD,
                PERSISTENT_TTL_EXTEND_TO,
            );
            value
        }
        None => 0,
    }
}

/// Persist `value` as the running total for `metric` and refresh its TTL.
pub fn set_counter(env: &Env, metric: &Symbol, value: i128) {
    let key = DataKey::Counter(metric.clone());
    env.storage().persistent().set(&key, &value);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}
