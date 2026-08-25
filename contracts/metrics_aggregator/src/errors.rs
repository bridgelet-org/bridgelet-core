use soroban_sdk::contracterror;

// Error codes for MetricsAggregator occupy the 13000–13099 range.
// See contracts/ephemeral_account/src/errors.rs for the full namespace map.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// [`MetricsAggregator::initialize`] was called more than once.
    AlreadyInitialized = 13000,

    /// A state-changing operation was attempted before the contract was
    /// initialized (no admin is set, so nobody can be authorized).
    NotInitialized = 13001,

    /// The caller is not the admin set during
    /// [`MetricsAggregator::initialize`].
    Unauthorized = 13002,

    /// `increment` was called by an address that has not been authorized as a
    /// writer by the admin.
    UnauthorizedWriter = 13003,

    /// `increment` was called with an amount that is not strictly positive.
    ///
    /// Counters are monotonic by design: they may only ever grow, so zero and
    /// negative amounts are rejected rather than silently ignored.
    InvalidAmount = 13004,

    /// The increment would push the counter past `i128::MAX`.
    ///
    /// The stored value is left unchanged when this is returned.
    CounterOverflow = 13005,
}
