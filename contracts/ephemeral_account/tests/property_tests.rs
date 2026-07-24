//! Issue #166 / #58: Property-based tests with `proptest`.
//!
//! These tests generate randomized valid/invalid inputs and assert invariants
//! that must hold for every generated case:
//!
//! 1. `amount_never_negative_after_sweep` — after a successful sweep the tracked
//!    reserve amounts are never negative, regardless of the payment amounts.
//! 2. `expired_account_always_rejects_sweep` — once past the expiry ledger a
//!    sweep is always rejected with `Error::AccountExpired`.
//! 3. `double_initialize_always_fails` — a second `initialize` call always
//!    fails with `Error::AlreadyInitialized`.
//! 4. `past_expiry_always_rejects_init` — initializing with a past expiry
//!    always returns `Error::InvalidExpiry`.
//! 5. `future_expiry_always_succeeds` — initializing with a valid future
//!    expiry always succeeds (first call).
//! 6. `random_addresses_do_not_panic` — any arbitrary 32-byte address as
//!    creator/recovery/controller never causes a panic during init.
//! 7. `record_payment_various_amounts` — amounts in 1..=i128::MAX never
//!    cause a panic after a valid init.

use ephemeral_account::{
    AccountStatus, EphemeralAccountContract, EphemeralAccountContractClient, Error,
};
use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, BytesN, Env,
};

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, failure_persistence: None, ..ProptestConfig::default() })]

    // Invariant 1: the tracked reserve never goes negative after a sweep, for
    // any set of valid positive payment amounts (1..=10 assets).
    #[test]
    fn amount_never_negative_after_sweep(
        amounts in prop::collection::vec(1i128..=1_000_000_000_000i128, 1..=10)
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &controller,
            &Address::generate(&env),
        );

        for amount in amounts.iter() {
            let asset = Address::generate(&env);
            client.record_payment(amount, &asset);
        }

        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);

        prop_assert_eq!(client.get_status(), AccountStatus::Swept);
        prop_assert!(client.get_reserve_remaining() >= 0);
        prop_assert!(client.get_reserve_available() >= 0);
    }

    // Invariant 2: any sweep attempted at or after the expiry ledger is
    // rejected with Error::AccountExpired.
    #[test]
    fn expired_account_always_rejects_sweep(
        expiry_offset in 1u32..=50u32,
        past in 0u32..=50u32,
        amount in 1i128..=1_000_000_000_000i128,
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);

        let start = env.ledger().sequence();
        let expiry_ledger = start + expiry_offset;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &controller,
            &Address::generate(&env),
        );
        client.record_payment(&amount, &asset);

        // Move to at-or-after expiry. The advance is kept small so the contract
        // instance stays within its TTL and is not archived by the test host.
        env.ledger().set_sequence_number(expiry_ledger + past);

        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        let result = client.try_sweep(&destination, &auth_sig);

        prop_assert!(matches!(result, Err(Ok(Error::AccountExpired))));
    }

    // Invariant 3: initializing an already-initialized account always fails
    // with Error::AlreadyInitialized, for any valid future expiry.
    #[test]
    fn double_initialize_always_fails(offset in 1u32..=1_000_000u32) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + offset;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
            &Address::generate(&env),
        );

        let result = client.try_initialize(
            &creator,
            &(expiry_ledger + 1),
            &recovery,
            &Address::generate(&env),
            &Address::generate(&env),
        );

        prop_assert!(matches!(result, Err(Ok(Error::AlreadyInitialized))));
    }

    // Invariant 4: initializing with a past or current expiry always returns
    // Error::InvalidExpiry.
    #[test]
    fn past_expiry_always_rejects_init(
        past_offset in 0u32..=1_000_000u32,
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let current = env.ledger().sequence();
        // past_offset == 0 means expiry == current (not in future), which should fail
        let expiry_ledger = current.saturating_sub(past_offset);

        let result = client.try_initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
            &Address::generate(&env),
        );

        prop_assert!(
            matches!(result, Err(Ok(Error::InvalidExpiry))),
            "Expected InvalidExpiry for past/current expiry {} (current={}), got {:?}",
            expiry_ledger, current, result
        );
    }

    // Invariant 5: initializing with a valid future expiry always succeeds
    // (first call), regardless of the specific future ledger number.
    #[test]
    fn future_expiry_always_succeeds(offset in 1u32..=1_000_000u32) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + offset;

        let result = client.try_initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
            &Address::generate(&env),
        );

        prop_assert!(
            matches!(result, Ok(Ok(()))),
            "Expected Ok for future expiry {}, got {:?}",
            expiry_ledger, result
        );
    }

    // Invariant 6: arbitrary addresses as creator/recovery/controller never
    // cause a panic — they are valid Soroban addresses and should be accepted.
    #[test]
    fn random_addresses_do_not_panic(
        offset in 1u32..=10_000u32,
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        // Generate completely random addresses — these are always valid
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let admin = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + offset;

        // Should never panic — any valid Soroban address is accepted
        let result = client.try_initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &controller,
            &admin,
        );

        prop_assert!(result.is_ok(), "Initialize panicked with random addresses: {:?}", result);
    }

    // Invariant 7: recording payments with various valid amounts after a
    // successful init never causes a panic.
    #[test]
    fn record_payment_various_amounts(
        amounts in prop::collection::vec(1i128..=i128::MAX, 1..=5),
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
            &Address::generate(&env),
        );

        for amount in amounts.iter() {
            let asset = Address::generate(&env);
            let result = client.try_record_payment(amount, &asset);
            prop_assert!(
                result.is_ok(),
                "record_payment panicked for amount {}: {:?}",
                amount, result
            );
        }
    }
}
