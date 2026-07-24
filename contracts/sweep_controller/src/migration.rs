use soroban_sdk::{contracttype, BytesN, Env};

use crate::storage;

/// Storage schema version.  Stored on-chain under `StorageKey::StorageVersion`.
/// Bump this whenever the storage layout changes in a backwards-incompatible
/// way.  The `migrate()` function reads this value and runs any pending
/// migrations before the contract proceeds with its normal logic.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl StorageVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Packed representation for comparison: `major << 16 | minor << 8 | patch`.
    pub fn packed(&self) -> u32 {
        ((self.major & 0xFF) << 16) | ((self.minor & 0xFF) << 8) | (self.patch & 0xFF)
    }
}

/// The current storage schema version.  Bump this when adding new storage
/// keys or changing existing layouts.
pub const CURRENT_VERSION: StorageVersion = StorageVersion {
    major: 1,
    minor: 0,
    patch: 0,
};

/// Read the stored schema version from instance storage.
/// Returns `None` if no version has been stored yet (pre-migration contract).
pub fn get_storage_version(env: &Env) -> Option<StorageVersion> {
    env.storage().instance().get(&crate::storage::DataKey::StorageVersion)
}

/// Write the schema version to instance storage.
pub fn set_storage_version(env: &Env, version: &StorageVersion) {
    env.storage()
        .instance()
        .set(&crate::storage::DataKey::StorageVersion, version);
}

/// Run any pending storage migrations.  Should be called at the top of
/// every admin-facing function (initialize, upgrade, etc.) and can also
/// be called explicitly via a dedicated `migrate()` entry point.
///
/// Migration strategy:
/// 1. Read the current stored version (or default to 0.0.0 if unset).
/// 2. Compare against `CURRENT_VERSION`.
/// 3. Run each migration step in order, bumping the version after each.
/// 4. Store the final version.
///
/// # Returns
/// The (old, new) version pair, or `None` if no migration was needed.
pub fn migrate(env: &Env) -> Option<(StorageVersion, StorageVersion)> {
    let old_version = get_storage_version(env).unwrap_or(StorageVersion::new(0, 0, 0));

    if old_version.packed() >= CURRENT_VERSION.packed() {
        return None; // Already up-to-date
    }

    // ── Migration steps ────────────────────────────────────────────────
    // Add migration steps here as the schema evolves.  Each step should
    // be idempotent and should not fail if the migration has already been
    // applied (in case a partial migration was previously committed).

    // Example future migration:
    // if old_version.packed() < 2_0_0 {
    //     migrate_v2(env);
    //     set_storage_version(env, &StorageVersion::new(2, 0, 0));
    // }

    // Set final version
    set_storage_version(env, &CURRENT_VERSION);

    Some((old_version, CURRENT_VERSION))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_version_packed() {
        let v = StorageVersion::new(2, 3, 4);
        assert_eq!(v.packed(), (2 << 16) | (3 << 8) | 4);
    }

    #[test]
    fn current_version_is_1_0_0() {
        assert_eq!(CURRENT_VERSION.packed(), 1 << 16);
    }

    #[test]
    fn get_version_returns_none_before_migration() {
        let env = Env::default();
        let v = get_storage_version(&env);
        assert!(v.is_none());
    }
}
