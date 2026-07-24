use soroban_sdk::contracterror;

// Issue #248: error codes are namespaced per contract. See
// contracts/ephemeral_account/src/errors.rs for the full namespace map.
// This contract owns 3000-3999.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The supplied amount is zero or negative; only positive stroops are valid.
    InvalidAmount = 3000,

    /// A read operation was attempted before any base reserve was stored.
    ///
    /// Callers should check [`ReserveContract::has_base_reserve`] or use the
    /// `Option`-returning [`ReserveContract::get_base_reserve`] instead of
    /// any helper that returns a bare value.
    ReserveNotSet = 3001,

    /// The caller is not the admin set during initialization.
    ///
    /// Only the admin address provided in [`ReserveContract::initialize`] may
    /// call state-changing operations such as [`ReserveContract::set_base_reserve`].
    Unauthorized = 3002,

    /// [`ReserveContract::initialize`] was called more than once.
    ///
    /// The contract may only be initialized once; subsequent calls are rejected
    /// to prevent admin takeover.
    AlreadyInitialized = 3003,

    /// A state-changing operation was attempted before [`ReserveContract::initialize`]
    /// was called.
    NotInitialized = 3004,

    /// The supplied amount exceeds the maximum allowed value.
    ///
    /// An upper bound prevents accidental misconfiguration
    /// (e.g. storing a value in XLM instead of stroops).
    /// Current ceiling: 10,000 XLM = 100_000_000_000 stroops.
    AmountTooLarge = 3005,
}
