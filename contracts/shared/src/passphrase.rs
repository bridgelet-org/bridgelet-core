use soroban_sdk::{Bytes, Env};

// Well-known network passphrase constants (32-byte SHA-256 hashes are
// stored on-chain as `Bytes<32>`; the human-readable strings are what
// wallets sign against — we hash them here for comparison).

/// Stellar Public Network passphrase
pub const PUBLIC_NETWORK_PASSPHRASE: &str = "Public Global Stellar Network ; September 2015";

/// Stellar Test Network passphrase
pub const TESTNET_PASSPHRASE: &str = "Test SDF Network ; September 2015";

/// Standalone (local sandbox) passphrase
pub const STANDALONE_PASSPHRASE: &str = "Standalone Network ; February 2017";

/// Hash a human-readable passphrase into the `Bytes<32>` format stored by
/// `env.ledger().network_id()`.
fn hash_passphrase(env: &Env, passphrase: &str) -> soroban_sdk::BytesN<32> {
    let bytes = Bytes::from_slice(env, passphrase.as_bytes());
    env.crypto().sha256(&bytes).into()
}

/// Verify that the current ledger's network passphrase matches one of the
/// expected values.  Returns `Ok(())` on match, or `Err(expected_hash)`
/// on mismatch so the caller can surface a meaningful error.
///
/// # Usage in `initialize()`
/// ```ignore
/// passphrase::require_network(&env, passphrase::TESTNET_PASSPHRASE)?;
/// ```
pub fn require_network(
    env: &Env,
    expected_passphrase: &str,
) -> Result<(), soroban_sdk::BytesN<32>> {
    let actual = env.ledger().network_id();
    let expected = hash_passphrase(env, expected_passphrase);
    if actual == expected {
        Ok(())
    } else {
        Err(expected)
    }
}

/// Return the 32-byte ledger network id for the standalone passphrase.
///
/// `Env::default()` in the Soroban test host initializes the ledger with an
/// all-zero network id rather than the standalone passphrase, so tests that
/// exercise `require_network` (which `initialize` enforces) must set the
/// ledger network id via `env.ledger().set_network_id(...)` before calling.
pub fn standalone_network_id(env: &Env) -> [u8; 32] {
    hash_passphrase(env, STANDALONE_PASSPHRASE).to_array()
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Ledger as _;

    #[test]
    fn test_hash_passphrase_deterministic() {
        let env = Env::default();
        let h1 = hash_passphrase(&env, TESTNET_PASSPHRASE);
        let h2 = hash_passphrase(&env, TESTNET_PASSPHRASE);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_different_passphrases_produce_different_hashes() {
        let env = Env::default();
        let h_pub = hash_passphrase(&env, PUBLIC_NETWORK_PASSPHRASE);
        let h_test = hash_passphrase(&env, TESTNET_PASSPHRASE);
        assert_ne!(h_pub, h_test);
    }

    #[test]
    fn test_require_network_passes_with_standalone_network_id() {
        // Env::default() starts with an all-zero network id, so tests must
        // opt in to the standalone passphrase before require_network passes.
        let env = Env::default();
        env.ledger().set_network_id(standalone_network_id(&env));
        assert!(require_network(&env, STANDALONE_PASSPHRASE).is_ok());
    }

    #[test]
    fn test_require_network_fails_on_wrong_passphrase() {
        let env = Env::default();
        let result = require_network(&env, TESTNET_PASSPHRASE);
        assert!(result.is_err());
    }
}
