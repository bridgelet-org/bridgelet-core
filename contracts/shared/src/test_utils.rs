/// Shared test utilities for Soroban test harness setup.
///
/// Import this module in both contracts' test modules to avoid
/// duplicating boilerplate setup code.
///
/// # Usage
/// ```ignore
/// #[cfg(test)]
/// mod test {
///     use bridgelet_shared::test_utils::*;
///     // Now you can call setup_env(), create_test_accounts(), etc.
/// }
/// ```
#[cfg(test)]
pub mod test_utils {
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env};

    /// Create a pre-configured `Env` suitable for contract testing.
    ///
    /// - Enables mock auth for all callers (no real signing needed).
    /// - Returns the environment.
    pub fn setup_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    /// Create `n` unique test addresses, each automatically funded
    /// by the Soroban test harness.
    pub fn create_test_accounts(env: &Env, n: usize) -> std::vec::Vec<Address> {
        (0..n).map(|_| Address::generate(env)).collect()
    }

    /// Generate a single test address.
    pub fn random_address(env: &Env) -> Address {
        Address::generate(env)
    }

    /// Create a test address pair (useful for from/to patterns).
    pub fn address_pair(env: &Env) -> (Address, Address) {
        (Address::generate(env), Address::generate(env))
    }

    /// Create a test address triple (useful for creator/destination/recovery).
    pub fn address_triple(env: &Env) -> (Address, Address, Address) {
        (
            Address::generate(env),
            Address::generate(env),
            Address::generate(env),
        )
    }

    /// Return a fixed-size byte array (useful for Ed25519 public keys).
    pub fn fixed_bytes_32() -> [u8; 32] {
        [0xAB; 32]
    }

    /// Return a zero-filled 64-byte array (useful for dummy signatures).
    pub fn zero_signature_64() -> [u8; 64] {
        [0u8; 64]
    }

    /// Return a 32-byte array with a specific seed (useful for deterministic keys).
    pub fn seeded_bytes_32(seed: u8) -> [u8; 32] {
        [seed; 32]
    }
}

#[cfg(test)]
mod tests {
    use super::test_utils::*;

    #[test]
    fn setup_env_creates_usable_environment() {
        let env = setup_env();
        let addr = random_address(&env);
        // Should not panic — address is valid and funded
        assert!(!addr.to_string().is_empty());
    }

    #[test]
    fn create_test_accounts_generates_correct_count() {
        let env = setup_env();
        let accounts = create_test_accounts(&env, 5);
        assert_eq!(accounts.len(), 5);
    }

    #[test]
    fn address_pair_returns_distinct_addresses() {
        let env = setup_env();
        let (a, b) = address_pair(&env);
        assert_ne!(a, b);
    }

    #[test]
    fn address_triple_returns_three_distinct_addresses() {
        let env = setup_env();
        let (a, b, c) = address_triple(&env);
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    #[test]
    fn fixed_bytes_32_has_correct_length() {
        assert_eq!(fixed_bytes_32().len(), 32);
    }

    #[test]
    fn zero_signature_64_has_correct_length() {
        assert_eq!(zero_signature_64().len(), 64);
    }

    #[test]
    fn seeded_bytes_32_produces_deterministic_output() {
        let a = seeded_bytes_32(0x42);
        let b = seeded_bytes_32(0x42);
        assert_eq!(a, b);
    }

    #[test]
    fn seeded_bytes_32_different_seeds_produce_different_output() {
        let a = seeded_bytes_32(0x01);
        let b = seeded_bytes_32(0x02);
        assert_ne!(a, b);
    }
}
