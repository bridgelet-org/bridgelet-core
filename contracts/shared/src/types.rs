use soroban_sdk::{contracttype, Address, Bytes, Vec};

/// Represents a payment received by the ephemeral account.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Payment {
    pub asset: Address,
    pub amount: i128,
    pub timestamp: u64,
}

/// The current status of an ephemeral account.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
#[repr(u32)]
pub enum AccountStatus {
    Active = 0,
    PaymentReceived = 1,
    Swept = 2,
    Expired = 3,
}

/// Account information structure
#[derive(Clone)]
#[contracttype]
pub struct AccountInfo {
    pub creator: Address,
    pub status: AccountStatus,
    pub expiry_ledger: u32,
    pub recovery_address: Address,
    pub payment_received: bool,
    pub payment_count: u32,
    pub payments: Vec<Payment>,
    pub swept_to: Option<Address>,
}

/// Request to initialize a single ephemeral account
#[contracttype]
#[derive(Clone, Debug)]
pub struct AccountInitRequest {
    pub expiry_ledger: u32,
    pub recovery_address: Address,
}

/// Result of initializing an ephemeral account
#[contracttype]
#[derive(Clone, Debug)]
pub struct AccountInitResult {
    pub account_address: Address,
    pub success: bool,
    pub error: Option<Bytes>,
}

/// Payload sent off-chain to authorize a sweep operation.
///
/// The SDK serialises this, signs it with the authorised Ed25519 key, and
/// passes the resulting signature to `execute_sweep()` or `claim()`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SweepPayload {
    /// Address of the ephemeral account to sweep.
    pub ephemeral_account: Address,
    /// Destination wallet that should receive the funds.
    pub destination: Address,
    /// Monotonic nonce — must match the on-chain nonce at verification time.
    pub nonce: u64,
    /// The network passphrase the signature was created against.
    pub network_passphrase: Bytes,
}

/// Snapshot of a single asset balance held by an ephemeral account.
///
/// Useful for SDK-level fee estimation and pre-sweep validation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetBalance {
    /// SEP-41 token contract address.
    pub asset: Address,
    /// Balance in the token's smallest unit (stroops for native XLM).
    pub balance: i128,
}

/// Version metadata stored on-chain to support forward-compatible
/// storage migrations (see StorageVersion key).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractVersion {
    /// Major version — breaking storage changes.
    pub major: u32,
    /// Minor version — backward-compatible additions.
    pub minor: u32,
    /// Patch version — bug fixes only.
    pub patch: u32,
}

impl ContractVersion {
    /// Returns a `u32` packed as `major << 16 | minor << 8 | patch`.
    pub fn packed(&self) -> u32 {
        ((self.major & 0xFF) << 16) | ((self.minor & 0xFF) << 8) | (self.patch & 0xFF)
    }

    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn initial() -> Self {
        Self::new(1, 0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_version_packed() {
        let v = ContractVersion::new(2, 3, 4);
        assert_eq!(v.packed(), (2 << 16) | (3 << 8) | 4);
    }

    #[test]
    fn contract_version_initial() {
        let v = ContractVersion::initial();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
        assert_eq!(v.packed(), 1 << 16);
    }
}
