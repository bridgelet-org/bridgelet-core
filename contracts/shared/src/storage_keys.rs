use soroban_sdk::contracttype;

/// Canonical storage key enum shared across all bridgelet contracts.
///
/// Centralising key names here prevents typos, makes namespace auditing
/// trivial, and ensures both contracts agree on the same storage layout
/// for cross-contract reads.
///
/// Each contract maps these to its own concrete `DataKey` enum (or reuses
/// this one directly) so that key values are consistent everywhere.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageKey {
    /// Whether the contract has been initialized.
    Initialized,
    /// Address that deployed / created the contract instance.
    Creator,
    /// Ledger number at which an ephemeral account expires.
    ExpiryLedger,
    /// Address to which funds are returned on expiry.
    RecoveryAddress,
    /// Map of recorded payments (asset → Payment).
    Payments,
    /// Current lifecycle status of the account.
    Status,
    /// Destination the account was swept (or expired) to.
    SweptTo,
    /// Remaining base reserve in stroops.
    BaseReserveRemaining,
    /// Base reserve currently available for transfer.
    AvailableReserve,
    /// Whether the base reserve has been fully reclaimed.
    ReserveReclaimed,
    /// Sequence number of the last sweep operation.
    LastSweepId,
    /// Number of reserve reclaim events emitted.
    ReserveEventCount,
    /// Most recent ReserveReclaimed event payload.
    LastReserveEvent,
    /// Address of the authorized sweep controller.
    AuthorizedController,
    /// Admin address for contract upgrades.
    Admin,
    /// Ed25519 public key of the authorized off-chain signer.
    AuthorizedSigner,
    /// Monotonic nonce used to prevent replay of sweep signatures.
    SweepNonce,
    /// Locked destination address (optional, for locked-mode controllers).
    AuthorizedDestination,
    /// WASM hash stored by AccountFactory for deploying ephemeral accounts.
    EphemeralAccountWasmHash,
    /// Schema version used for storage migration (see #146).
    StorageVersion,
}

// ── Convenience helpers ────────────────────────────────────────────────
// These return the enum variants wrapped in `soroban_sdk::Val` via
// `IntoVal` so callers can write `StorageKey::key_initialized()` instead
// of constructing the `DataKey` variant every time.  They are purely
// compile-time constants — no heap allocation.

impl StorageKey {
    pub fn key_initialized() -> Self {
        Self::Initialized
    }
    pub fn key_creator() -> Self {
        Self::Creator
    }
    pub fn key_expiry_ledger() -> Self {
        Self::ExpiryLedger
    }
    pub fn key_status() -> Self {
        Self::Status
    }
    pub fn key_payments() -> Self {
        Self::Payments
    }
    pub fn key_authorized_signer() -> Self {
        Self::AuthorizedSigner
    }
    pub fn key_sweep_nonce() -> Self {
        Self::SweepNonce
    }
    pub fn key_storage_version() -> Self {
        Self::StorageVersion
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that every variant has a distinct discriminant so Soroban
    /// storage does not silently alias unrelated entries.
    #[test]
    fn storage_key_variants_are_distinct() {
        let keys = [
            StorageKey::Initialized as u32,
            StorageKey::Creator as u32,
            StorageKey::ExpiryLedger as u32,
            StorageKey::RecoveryAddress as u32,
            StorageKey::Payments as u32,
            StorageKey::Status as u32,
            StorageKey::SweptTo as u32,
            StorageKey::BaseReserveRemaining as u32,
            StorageKey::AvailableReserve as u32,
            StorageKey::ReserveReclaimed as u32,
            StorageKey::LastSweepId as u32,
            StorageKey::ReserveEventCount as u32,
            StorageKey::LastReserveEvent as u32,
            StorageKey::AuthorizedController as u32,
            StorageKey::Admin as u32,
            StorageKey::AuthorizedSigner as u32,
            StorageKey::SweepNonce as u32,
            StorageKey::AuthorizedDestination as u32,
            StorageKey::EphemeralAccountWasmHash as u32,
            StorageKey::StorageVersion as u32,
        ];

        let mut sorted = keys;
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            keys.len(),
            "duplicate StorageKey discriminant detected"
        );
    }

    #[test]
    fn convenience_helpers_return_expected_variants() {
        assert_eq!(
            StorageKey::key_initialized(),
            StorageKey::Initialized
        );
        assert_eq!(StorageKey::key_creator(), StorageKey::Creator);
        assert_eq!(StorageKey::key_storage_version(), StorageKey::StorageVersion);
    }
}
