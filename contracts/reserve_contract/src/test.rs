#[cfg(test)]
mod test {
    extern crate std;

    use crate::{ReserveContract, ReserveContractClient};
    use soroban_sdk::{
        testutils::{storage::Instance as _, Address as _},
        Address, Env,
    };

    use soroban_sdk::testutils::Ledger;

    // HELPERS

    /// Build a test `Env` with ledger settings that let TTL extension reach
    /// `INSTANCE_TTL_EXTEND_TO` (518 400 ledgers) without being capped:
    ///
    /// * `min_persistent_entry_ttl = 50` — below `INSTANCE_TTL_THRESHOLD` (100)
    ///   so a freshly deployed instance always has TTL < threshold and
    ///   `extend_ttl` fires on the very first call.
    /// * `max_entry_ttl = 600_000` — well above 518 400 so the ledger cap
    ///   never clips the extension.
    fn create_env() -> Env {
        let env = Env::default();
        env.ledger().with_mut(|li| {
            li.sequence_number = 100_000;
            li.min_persistent_entry_ttl = 50;
            li.min_temp_entry_ttl = 50;
            li.max_entry_ttl = 600_000;
        });
        env
    }

    /// Deploy a fresh ReserveContract, initialize it with a random admin, and
    /// return `(env, client, admin, contract_id)`.
    fn setup() -> (Env, ReserveContractClient<'static>, Address, Address) {
        let env = create_env();
        env.mock_all_auths();
        let contract_id = env.register(ReserveContract, ());
        let client = ReserveContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, client, admin, contract_id)
    }

    /// Deploy a fresh ReserveContract **without** initializing it, and return
    /// `(env, client, contract_id)`.
    fn setup_uninitialized() -> (Env, ReserveContractClient<'static>, Address) {
        let env = create_env();
        env.mock_all_auths();
        let contract_id = env.register(ReserveContract, ());
        let client = ReserveContractClient::new(&env, &contract_id);
        (env, client, contract_id)
    }

    /// Assert that the most recent contract call extended the instance TTL to
    /// at least `INSTANCE_TTL_EXTEND_TO` (518 400 ledgers).
    fn assert_ttl_extended(env: &Env, contract_id: &Address) {
        let ttl = env.as_contract(contract_id, || env.storage().instance().get_ttl());
        assert!(
            ttl >= 518_400,
            "TTL should be at least 518_400 ledgers, got {ttl}"
        );
    }

    //  Initialization

    /// initialize() stores the admin and get_admin() returns it.
    #[test]
    fn test_initialize_stores_admin() {
        let (env, _, _admin, _) = setup();
        let contract_id = env.register(ReserveContract, ());
        let client = ReserveContractClient::new(&env, &contract_id);
        let new_admin = Address::generate(&env);
        client.initialize(&new_admin);
        assert_eq!(client.get_admin(), Some(new_admin));
        assert_ttl_extended(&env, &contract_id);
    }

    /// Double initialization must fail with error #4 (AlreadyInitialized).
    #[test]
    #[should_panic(expected = "Error(Contract, #4)")]
    fn test_initialize_twice_panics() {
        let (env, client, _admin, _) = setup();
        let another = Address::generate(&env);
        client.initialize(&another);
    }

    //  Not-initialized guard

    /// set_base_reserve must fail with error #5 (NotInitialized) on a fresh
    /// contract that was never initialized.
    #[test]
    #[should_panic(expected = "Error(Contract, #5)")]
    fn test_set_base_reserve_before_initialize_panics() {
        let (_env, client, _) = setup_uninitialized();
        client.set_base_reserve(&1_000_000_000i128);
    }

    //  Safe-default handling (reads don't require init)

    /// Before anything is stored, get_base_reserve() must return None.
    #[test]
    fn test_get_base_reserve_returns_none_when_not_set() {
        let (env, client, contract_id) = setup_uninitialized();
        assert_eq!(client.get_base_reserve(), None);
        assert_ttl_extended(&env, &contract_id);
    }

    /// has_base_reserve() must be false on a fresh contract.
    #[test]
    fn test_has_base_reserve_returns_false_when_not_set() {
        let (env, client, contract_id) = setup_uninitialized();
        assert!(!client.has_base_reserve());
        assert_ttl_extended(&env, &contract_id);
    }

    /// require_base_reserve() must panic (contract error #2) when not set.
    #[test]
    #[should_panic(expected = "Error(Contract, #2)")]
    fn test_require_base_reserve_panics_when_not_set() {
        let (_env, client, _) = setup_uninitialized();
        client.require_base_reserve();
    }

    //  Set / get round-trip

    /// A stored value must be returned verbatim by all three read functions.
    #[test]
    fn test_set_and_get_base_reserve() {
        let (env, client, _admin, contract_id) = setup();

        // 100 XLM expressed in stroops (1 XLM = 10_000_000 stroops)
        let reserve = 1_000_000_000i128;
        client.set_base_reserve(&reserve);

        assert_eq!(client.get_base_reserve(), Some(reserve));
        assert!(client.has_base_reserve());
        assert_eq!(client.require_base_reserve(), reserve);
        assert_ttl_extended(&env, &contract_id);
    }

    /// The minimum meaningful value (1 stroop) must be accepted.
    #[test]
    fn test_set_base_reserve_minimum_valid_value() {
        let (env, client, _admin, contract_id) = setup();
        client.set_base_reserve(&1i128);
        assert_eq!(client.get_base_reserve(), Some(1i128));
        assert_ttl_extended(&env, &contract_id);
    }

    //  Overwrite behaviour

    /// set_base_reserve() must overwrite the previous value.
    #[test]
    fn test_set_base_reserve_overwrites_previous_value() {
        let (env, client, _admin, contract_id) = setup();

        client.set_base_reserve(&1_000_000_000i128);
        assert_eq!(client.get_base_reserve(), Some(1_000_000_000i128));

        client.set_base_reserve(&2_000_000_000i128);
        assert_eq!(client.get_base_reserve(), Some(2_000_000_000i128));

        assert!(client.has_base_reserve());
        assert_ttl_extended(&env, &contract_id);
    }

    //  Input validation

    /// Zero is not a valid reserve; the contract must reject it with error #1.
    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_set_base_reserve_zero_is_rejected() {
        let (_env, client, _admin, _) = setup();
        client.set_base_reserve(&0i128);
    }

    /// Negative amounts are nonsensical and must be rejected with error #1.
    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_set_base_reserve_negative_is_rejected() {
        let (_env, client, _admin, _) = setup();
        client.set_base_reserve(&-1i128);
    }

    /// A large negative amount (i128::MIN) must also be rejected.
    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_set_base_reserve_min_i128_is_rejected() {
        let (_env, client, _admin, _) = setup();
        client.set_base_reserve(&i128::MIN);
    }

    //  Range validation (upper bound)

    /// The maximum allowed value (10 000 XLM = 100_000_000_000 stroops)
    /// must be accepted.
    #[test]
    fn test_set_base_reserve_at_max_is_accepted() {
        let (env, client, _admin, contract_id) = setup();
        let max = 100_000_000_000i128;
        client.set_base_reserve(&max);
        assert_eq!(client.get_base_reserve(), Some(max));
        assert_ttl_extended(&env, &contract_id);
    }

    /// One stroop above the ceiling must be rejected with error #6.
    #[test]
    #[should_panic(expected = "Error(Contract, #6)")]
    fn test_set_base_reserve_above_max_is_rejected() {
        let (_env, client, _admin, _) = setup();
        client.set_base_reserve(&100_000_000_001i128);
    }

    /// An absurdly large value must be rejected with error #6.
    #[test]
    #[should_panic(expected = "Error(Contract, #6)")]
    fn test_set_base_reserve_huge_value_is_rejected() {
        let (_env, client, _admin, _) = setup();
        client.set_base_reserve(&i128::MAX);
    }

    //  State isolation

    /// Two independently deployed instances share no state.
    #[test]
    fn test_two_contracts_are_independent() {
        let env = create_env();
        env.mock_all_auths();

        let id_a = env.register(ReserveContract, ());
        let id_b = env.register(ReserveContract, ());

        let client_a = ReserveContractClient::new(&env, &id_a);
        let client_b = ReserveContractClient::new(&env, &id_b);

        let admin_a = Address::generate(&env);
        let admin_b = Address::generate(&env);
        client_a.initialize(&admin_a);
        client_b.initialize(&admin_b);

        client_a.set_base_reserve(&500_000_000i128);

        // Contract B must still be unset.
        assert_eq!(client_b.get_base_reserve(), None);
        assert!(!client_b.has_base_reserve());

        // Contract A's value is unchanged.
        assert_eq!(client_a.get_base_reserve(), Some(500_000_000i128));

        // Both instances must have had their TTL extended.
        assert_ttl_extended(&env, &id_a);
        assert_ttl_extended(&env, &id_b);
    }

    //  Admin accessor

    /// get_admin returns None before initialization.
    #[test]
    fn test_get_admin_returns_none_before_init() {
        let (env, client, contract_id) = setup_uninitialized();
        assert_eq!(client.get_admin(), None);
        assert_ttl_extended(&env, &contract_id);
    }

    /// get_admin returns the admin after initialization.
    #[test]
    fn test_get_admin_returns_admin_after_init() {
        let (env, client, admin, contract_id) = setup();
        assert_eq!(client.get_admin(), Some(admin));
        assert_ttl_extended(&env, &contract_id);
    }

    //  TTL management

    /// After any interaction the instance TTL should be extended.
    /// We verify by reading the TTL inside the contract context and
    /// asserting it is at least the INSTANCE_TTL_EXTEND_TO value.
    #[test]
    fn test_ttl_extended_after_read() {
        let env = create_env();
        env.mock_all_auths();
        let contract_id = env.register(ReserveContract, ());
        let client = ReserveContractClient::new(&env, &contract_id);

        // Even a simple read should extend TTL.
        let _ = client.get_base_reserve();

        assert_ttl_extended(&env, &contract_id);
    }

    /// After initialize + set_base_reserve the TTL must still be alive.
    #[test]
    fn test_ttl_extended_after_write() {
        let env = create_env();
        env.mock_all_auths();
        let contract_id = env.register(ReserveContract, ());
        let client = ReserveContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        client.set_base_reserve(&5_000_000i128);

        assert_ttl_extended(&env, &contract_id);
    }
}
