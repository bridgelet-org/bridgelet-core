use soroban_sdk::{Bytes, Env};

/// well-known network passphrase constants (32-byte SHA-256 hashes are
/// stored on-chain as `Bytes<32>`; the human-readable strings are what
/// wallets sign against — we hash them here for comparison).

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
    env.crypto().sha256(&bytes)
}

/// Verify that the current ledger's network passphrase matches one of the
/// expected values.  Returns `Ok(())` on match, or `Err(expected_hash)`
/// on mismatch so the caller can surface a meaningful error.
///
/// # Usage in `initialize()`
/// ```ignore
/// passphrase::require_network(&env, passphrase::TESTNET_PASSPHRASE)?;
/// ```
pub fn require_network(env: &Env, expected_passphrase: &str) -> Result<(), soroban_sdk::BytesN<32>> {
    let actual = env.ledger().network_id();
    let expected = hash_passphrase(env, expected_passphrase);
    if actual == expected {
        Ok(())
    } else {
        Err(expected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_require_network_passes_in_default_env() {
        // Env::default() uses the standalone passphrase.
        let env = Env::default();
        assert!(require_network(&env, STANDALONE_PASSPHRASE).is_ok());
    }

    #[test]
    fn test_require_network_fails_on_wrong_passphrase() {
        let env = Env::default();
        let result = require_network(&env, TESTNET_PASSPHRASE);
        assert!(result.is_err());
    }
}
