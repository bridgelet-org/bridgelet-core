#[cfg(test)]
mod test {
    extern crate std;
    use crate::{
        storage, AccountStatus, EphemeralAccountContract, EphemeralAccountContractClient,
        ReserveReclaimed,
    };
    use soroban_sdk::testutils::Ledger;
    use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

    const BASE_RESERVE_STROOPS: i128 = 1_000_000_000;
    fn latest_reserve_event(client: &EphemeralAccountContractClient) -> ReserveReclaimed {
        client
            .get_last_reserve_event()
            .expect("reserve event was not emitted")
    }

    /// Register a Stellar asset contract and return its address.
    /// Mints `amount` tokens to `recipient`.
    fn make_token(env: &Env, amount: i128, recipient: &Address) -> Address {
        let admin = Address::generate(env);
        let token = env.register_stellar_asset_contract_v2(admin.clone());
        soroban_sdk::token::StellarAssetClient::new(env, &token.address()).mint(recipient, &amount);
        token.address()
    }

    #[test]
    fn test_initialize() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let relayer = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &relayer);
        let signer = test_signer_pubkey(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &signer, &1i128);

        assert_eq!(client.get_status(), AccountStatus::Active);
        assert!(!client.is_expired());
        assert_eq!(client.get_reserve_remaining(), BASE_RESERVE_STROOPS);
        assert_eq!(client.get_reserve_available(), BASE_RESERVE_STROOPS);
        assert!(!client.is_reserve_reclaimed());
    }
    #[test]
    fn test_version_stored_on_initialize() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let relayer = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &relayer);
        let signer = test_signer_pubkey(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;
        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &1i128);
        assert_eq!(client.version(), 1);
    }
    #[test]
    fn test_record_payment() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let asset = Address::generate(&env);

        client.record_payment(&100, &asset);
        assert_eq!(client.get_status(), AccountStatus::PaymentReceived);
    }
    #[test]
    #[should_panic(expected = "Error(Contract, #3)")]
    fn test_second_payment_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let asset1 = Address::generate(&env);
        let asset2 = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;
        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &1i128);

        client.record_payment(&100, &asset1);
        client.record_payment(&50, &asset2);
        let info = client.get_info();
        assert_eq!(info.payment_count, 2);
        assert_eq!(client.get_status(), AccountStatus::PaymentReceived);
    }

    /// Issue #106: sweep() calls token::TokenClient::transfer() for each recorded
    /// payment, moving funds from the contract to the destination on-chain.
    #[test]
    fn test_sweep_executes_token_transfer() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;
        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &1i128);

        let asset = make_token(&env, 500, &contract_id);
        let token_client = soroban_sdk::token::TokenClient::new(&env, &asset);
        assert_eq!(token_client.balance(&contract_id), 500);

        client.record_payment(&500, &asset);
        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);

        assert_eq!(token_client.balance(&contract_id), 0);
        assert_eq!(token_client.balance(&destination), 500);
        assert_eq!(client.get_status(), AccountStatus::Swept);
    }

    #[test]
    fn test_sweep_single_asset() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;
        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &1i128);

        let asset = make_token(&env, 100, &contract_id);
        client.record_payment(&100, &asset);
        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);
        assert_eq!(client.get_status(), AccountStatus::Swept);
        assert_eq!(client.get_reserve_remaining(), 0);
        assert!(client.is_reserve_reclaimed());
        let reserve_event = latest_reserve_event(&client);
        assert_eq!(reserve_event.destination, destination);
        assert_eq!(reserve_event.amount, BASE_RESERVE_STROOPS);
        assert_eq!(reserve_event.remaining_reserve, 0);
        assert!(reserve_event.fully_reclaimed);
        assert_eq!(reserve_event.sweep_id, env.ledger().sequence() as u64);
        assert_eq!(client.get_reserve_reclaim_event_count(), 1);
    }
    /// Issue #105: a sweep with a different destination must revert with
    /// Error::SweepDestinationLocked (#15).
    #[test]
    #[should_panic(expected = "Error(Contract, #16)")]
    fn test_sweep_destination_locked() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let signer = test_signer_pubkey(&env);
        let asset = Address::generate(&env);
        let dest1 = Address::generate(&env);
        let dest2 = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &signer, &1i128);
        client.record_payment(&100, &asset);
        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        // First sweep locks dest1
        client.sweep(&dest1, &auth_sig);
        // Second sweep with a different destination must revert (#15)
        // (AlreadySwept would fire first; test a pre-sweep scenario via a fresh
        // account where destination is set but status is not yet Swept, which
        // is handled by setting the key directly)
        let contract_id2 = env.register(EphemeralAccountContract, ());
        let client2 = EphemeralAccountContractClient::new(&env, &contract_id2);
        client2.initialize(&creator, &expiry_ledger, &recovery, &controller, &1i128);
        client2.record_payment(&100, &asset);
        // Pre-lock dest1 without sweeping
        env.as_contract(&contract_id2, || {
            storage::set_sweep_destination(&env, &dest1);
        });
        // Now sweep with dest2 — must revert with SweepDestinationLocked (#15)
        client2.sweep(&dest2, &auth_sig);
    }
    #[test]
    fn test_duplicate_asset() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, _, _, _, client) = setup(&env);
        let asset = Address::generate(&env);
        client.record_payment(&100, &asset);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.record_payment(&50, &asset);
        }));
        assert!(result.is_err());
    }
    #[test]
    #[should_panic(expected = "Error(Contract, #14)")]
    fn test_too_many_assets() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let relayer = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;
        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &1i128);

        for i in 0..10 {
            let asset = Address::generate(&env);
            client.record_payment(&(100 + i as i128), &asset);
        }
        let asset = Address::generate(&env);
        client.record_payment(&200, &asset);
    }
    #[test]
    fn test_sweep_reclaims_base_reserve_success_lifecycle() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &1i128);

        let asset1 = make_token(&env, 100, &contract_id);
        let asset2 = make_token(&env, 200, &contract_id);
        client.record_payment(&100, &asset1);
        client.record_payment(&200, &asset2);
        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);
        assert_eq!(client.get_status(), AccountStatus::Swept);
        assert_eq!(client.get_reserve_remaining(), 0);
        assert!(client.is_reserve_reclaimed());
        let reserve_event = latest_reserve_event(&client);
        assert_eq!(reserve_event.destination, destination);
        assert_eq!(reserve_event.amount, BASE_RESERVE_STROOPS);
        assert_eq!(reserve_event.remaining_reserve, 0);
        assert!(reserve_event.fully_reclaimed);
        assert_eq!(client.get_reserve_reclaim_event_count(), 1);
    }
    #[test]
    fn test_reserve_double_claim_prevention() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &1i128);

        let asset = make_token(&env, 100, &contract_id);
        client.record_payment(&100, &asset);
        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);
        assert_eq!(client.get_reserve_remaining(), 0);
        assert!(client.is_reserve_reclaimed());
        let reclaimed_again = client.reclaim_reserve();
        assert_eq!(reclaimed_again, 0);
        let reserve_event = latest_reserve_event(&client);
        assert_eq!(reserve_event.destination, destination);
        assert_eq!(reserve_event.amount, 0);
        assert!(reserve_event.fully_reclaimed);
        assert_eq!(client.get_reserve_reclaim_event_count(), 2);
    }
    #[test]
    fn test_reserve_reclaim_insufficient_balance_lifecycle() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let relayer = Address::generate(&env);
        let signer = test_signer_pubkey(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &1i128);

        let asset = make_token(&env, 100, &contract_id);
        client.record_payment(&100, &asset);
        let initial_available = 250_000_000i128;
        env.as_contract(&contract_id, || {
            storage::set_available_reserve(&env, initial_available);
        });
        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);
        let expected_remaining = BASE_RESERVE_STROOPS - initial_available;
        assert_eq!(client.get_status(), AccountStatus::Swept);
        assert_eq!(client.get_reserve_remaining(), expected_remaining);
        assert_eq!(client.get_reserve_available(), 0);
        assert!(!client.is_reserve_reclaimed());

        let no_balance_reclaim = client.reclaim_reserve();
        assert_eq!(no_balance_reclaim, 0);

        env.as_contract(&contract_id, || {
            storage::set_available_reserve(&env, expected_remaining);
        });
        let final_reclaim = client.reclaim_reserve();
        assert_eq!(final_reclaim, expected_remaining);
        assert_eq!(client.get_reserve_remaining(), 0);
        assert!(client.is_reserve_reclaimed());

        let noop = client.reclaim_reserve();
        assert_eq!(noop, 0);
        assert_eq!(client.get_reserve_reclaim_event_count(), 4);
    }

    /// With single-payment restriction, only one payment is recorded per account.
    /// Verifies that expire() handles i128::MAX amount without overflow.
    #[test]
    fn test_expire_overflow_protection() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let relayer = Address::generate(&env);
        let signer = test_signer_pubkey(&env);
        let asset1 = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &relayer);
        client.initialize(&creator, &expiry_ledger, &recovery, &signer, &1i128);

        // Single payment with MAX amount — no overflow possible with one payment
        client.record_payment(&i128::MAX, &asset1);

        // Advance past expiry
        env.ledger()
            .with_mut(|l| l.sequence_number = expiry_ledger + 1);

        // expire() must succeed: single i128::MAX payment has no overflow risk
        client.expire();
        assert_eq!(client.get_status(), AccountStatus::Expired);
    }

    #[test]
    fn test_replay_sweep_call_does_not_reclaim_twice() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &1i128);

        let asset = make_token(&env, 100, &contract_id);
        client.record_payment(&100, &asset);
        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);
        // `setup()` already initializes and records a sweep to get destination locked
        let reserve_events_before = client.get_reserve_reclaim_event_count();
        let replay = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.sweep(&destination, &auth_sig);
        }));
        assert!(replay.is_err());
        assert_eq!(client.get_status(), AccountStatus::Swept);
        assert_eq!(client.get_reserve_remaining(), 0);
        assert_eq!(
            client.get_reserve_reclaim_event_count(),
            reserve_events_before
        );
    }
    #[test]
    fn test_expire_returns_funds_to_recovery() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let relayer = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 10;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &relayer);
        let signer = test_signer_pubkey(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 10;
        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &1i128);
        client.record_payment(&500, &asset);
        env.ledger().with_mut(|l| {
            l.sequence_number = expiry_ledger + 1;
        });
        assert!(client.is_expired());
        client.expire();
        assert_eq!(client.get_status(), AccountStatus::Expired);
        let reserve_event = latest_reserve_event(&client);
        assert_eq!(reserve_event.destination, recovery);
        assert!(reserve_event.fully_reclaimed);
    }
    #[test]
    #[should_panic(expected = "Error(Contract, #6)")]
    fn test_expire_before_expiry_ledger_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let relayer = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;
        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &1i128);

        // Attempt to expire before expiry ledger — should return NotExpired (#6)
        client.expire();
    }
    #[test]
    #[should_panic(expected = "Error(Contract, #15)")]
    fn test_payment_below_minimum_rejected() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let signer = test_signer_pubkey(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &signer, &100i128);

        // Payment of 50 is below minimum of 100 -- should panic with PaymentBelowMinimum (#15)
        client.record_payment(&50, &asset);
    }

    #[test]
    fn test_payment_at_minimum_accepted() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let signer = test_signer_pubkey(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &signer, &100i128);

        // Payment exactly at minimum should succeed
        client.record_payment(&100, &asset);
        assert_eq!(client.get_status(), AccountStatus::PaymentReceived);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #7)")]
    fn test_sweep_after_already_swept_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let destination = Address::generate(&env);

        client.initialize(&creator, &expiry_ledger, &recovery, &controller, &1i128);

        let asset = make_token(&env, 100, &contract_id);
        client.record_payment(&100, &asset);
        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);
        client.sweep(&destination, &auth_sig);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5)")]
    fn test_initialize_with_expired_ledger_rejected() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let relayer = Address::generate(&env);
        let signer = test_signer_pubkey(&env);

        // Advance ledger so we can clearly pass a past expiry
        env.ledger().with_mut(|l| {
            l.sequence_number = 100;
        });

        // expiry_ledger <= current ledger (50 <= 100) -- should return InvalidExpiry (#5)
        let expired_ledger = 50u32;
        client.initialize(&creator, &expired_ledger, &recovery, &signer, &1i128);
    }
}
