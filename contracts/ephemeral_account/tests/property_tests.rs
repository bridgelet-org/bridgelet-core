//! Issue #167 / #58: Property-based tests with `proptest`.
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
}
