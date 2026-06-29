#[cfg(test)]
mod test {
    extern crate std;

    use std::println;

    use crate::{
        storage, AccountStatus, EphemeralAccountContract, EphemeralAccountContractClient, Error,
        ReserveReclaimed,
    };
    use soroban_sdk::{
        testutils::{Address as _, Ledger as _},
        Address, BytesN, Env, InvokeError,
    };

    const BASE_RESERVE_STROOPS: i128 = 1_000_000_000;

    fn latest_reserve_event(client: &EphemeralAccountContractClient) -> ReserveReclaimed {
        client
            .get_last_reserve_event()
            .expect("reserve event was not emitted")
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
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller);

        assert_eq!(client.get_status(), AccountStatus::Active);
        assert!(!client.is_expired());
        assert_eq!(client.get_reserve_remaining(), BASE_RESERVE_STROOPS);
        assert_eq!(client.get_reserve_available(), BASE_RESERVE_STROOPS);
        assert!(!client.is_reserve_reclaimed());
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
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller);
        client.record_payment(&100, &asset);

        assert_eq!(client.get_status(), AccountStatus::PaymentReceived);
    }

    #[test]
    fn test_multiple_payments() {
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

        client.initialize(&creator, &expiry_ledger, &recovery, &controller);

        client.record_payment(&100, &asset1);
        let info = client.get_info();
        assert_eq!(info.payment_count, 1);

        client.record_payment(&50, &asset2);
        let info = client.get_info();
        assert_eq!(info.payment_count, 2);

        assert_eq!(client.get_status(), AccountStatus::PaymentReceived);
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
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller);
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

    #[test]
    fn test_duplicate_asset_returns_expected_error_code() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller);
        client.record_payment(&100, &asset);
        let result = client.try_record_payment(&50, &asset);

        assert!(matches!(result, Err(Ok(Error::DuplicateAsset))));
    }

    #[test]
    fn test_too_many_assets_returns_expected_error_code() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller);

        for i in 0..10 {
            let asset = Address::generate(&env);
            client.record_payment(&(100 + i as i128), &asset);
        }

        let asset = Address::generate(&env);
        let result = client.try_record_payment(&200, &asset);

        assert!(matches!(result, Err(Ok(Error::TooManyPayments))));
    }

    #[test]
    fn test_record_payment_returns_not_initialized_error() {
        let env = Env::default();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let asset = Address::generate(&env);
        let result = client.try_record_payment(&100, &asset);

        assert!(matches!(result, Err(Ok(Error::NotInitialized))));
    }

    #[test]
    fn test_record_payment_returns_invalid_amount_error() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
        );
        let result = client.try_record_payment(&0, &asset);

        assert!(matches!(result, Err(Ok(Error::InvalidAmount))));
    }

    #[test]
    fn test_initialize_returns_invalid_expiry_error() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence();

        let result = client.try_initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
        );

        assert!(matches!(result, Err(Ok(Error::InvalidExpiry))));
    }

    #[test]
    fn test_expire_returns_not_expired_error() {
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
        );
        let result = client.try_expire();

        assert!(matches!(result, Err(Ok(Error::NotExpired))));
    }

    #[test]
    fn test_sweep_returns_no_payment_received_error() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
        );
        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        let result = client.try_sweep(&destination, &auth_sig);

        assert!(matches!(result, Err(Ok(Error::NoPaymentReceived))));
    }

    #[test]
    fn test_sweep_returns_account_expired_error() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
        );
        client.record_payment(&100, &asset);
        env.ledger().set_sequence_number(expiry_ledger);

        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        let result = client.try_sweep(&destination, &auth_sig);

        assert!(matches!(result, Err(Ok(Error::AccountExpired))));
    }

    #[test]
    fn test_sweep_returns_already_swept_error() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
        );
        client.record_payment(&100, &asset);

        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);
        let replay_result = client.try_sweep(&destination, &auth_sig);

        assert!(matches!(replay_result, Err(Ok(Error::AlreadySwept))));
    }

    #[test]
    fn test_sweep_accepts_placeholder_authorization_and_succeeds() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
        );
        client.record_payment(&100, &asset);

        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        let result = client.try_sweep(&destination, &auth_sig);
        println!("sweep placeholder auth result: {:?}", result);

        assert!(matches!(result, Ok(Ok(()))));
    }

    #[test]
    fn test_error_variants_have_expected_numeric_codes() {
        assert_eq!(Error::AlreadyInitialized as u32, 1);
        assert_eq!(Error::NotInitialized as u32, 2);
        assert_eq!(Error::PaymentAlreadyReceived as u32, 3);
        assert_eq!(Error::InvalidAmount as u32, 4);
        assert_eq!(Error::InvalidExpiry as u32, 5);
        assert_eq!(Error::NotExpired as u32, 6);
        assert_eq!(Error::AlreadySwept as u32, 7);
        assert_eq!(Error::Unauthorized as u32, 8);
        assert_eq!(Error::InvalidSignature as u32, 9);
        assert_eq!(Error::NoPaymentReceived as u32, 10);
        assert_eq!(Error::AccountExpired as u32, 11);
        assert_eq!(Error::InvalidStatus as u32, 12);
        assert_eq!(Error::DuplicateAsset as u32, 13);
        assert_eq!(Error::TooManyPayments as u32, 14);
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

        client.initialize(&creator, &expiry_ledger, &recovery, &controller);

        let asset1 = Address::generate(&env);
        let asset2 = Address::generate(&env);
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
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller);
        client.record_payment(&100, &asset);

        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);

        assert_eq!(client.get_reserve_remaining(), 0);
        assert!(client.is_reserve_reclaimed());

        let reclaimed_again = client.reclaim_reserve();
        assert_eq!(reclaimed_again, 0);
        assert_eq!(client.get_reserve_remaining(), 0);

        let reserve_event = latest_reserve_event(&client);
        assert_eq!(reserve_event.destination, destination);
        assert_eq!(reserve_event.amount, 0);
        assert_eq!(reserve_event.remaining_reserve, 0);
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
        let destination = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller);
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

        let partial_event = latest_reserve_event(&client);
        assert_eq!(partial_event.destination, destination);
        assert_eq!(partial_event.amount, initial_available);
        assert_eq!(partial_event.remaining_reserve, expected_remaining);
        assert!(!partial_event.fully_reclaimed);

        let no_balance_reclaim = client.reclaim_reserve();
        assert_eq!(no_balance_reclaim, 0);
        assert_eq!(client.get_reserve_remaining(), expected_remaining);
        assert!(!client.is_reserve_reclaimed());

        env.as_contract(&contract_id, || {
            storage::set_available_reserve(&env, expected_remaining);
        });
        let final_reclaim = client.reclaim_reserve();
        assert_eq!(final_reclaim, expected_remaining);
        assert_eq!(client.get_reserve_remaining(), 0);
        assert!(client.is_reserve_reclaimed());

        let noop_after_full_reclaim = client.reclaim_reserve();
        assert_eq!(noop_after_full_reclaim, 0);
        assert_eq!(client.get_reserve_remaining(), 0);
        assert_eq!(client.get_reserve_reclaim_event_count(), 4);
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
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery, &controller);
        client.record_payment(&100, &asset);

        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);

        let reserve_events_before = client.get_reserve_reclaim_event_count();
        let replay_attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.sweep(&destination, &auth_sig);
        }));

        assert!(replay_attempt.is_err());
        assert_eq!(client.get_status(), AccountStatus::Swept);
        assert_eq!(client.get_reserve_remaining(), 0);
        assert!(client.is_reserve_reclaimed());
        assert_eq!(
            client.get_reserve_reclaim_event_count(),
            reserve_events_before
        );
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_double_initialize_is_rejected() {
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
        );
        client.initialize(
            &creator,
            &(expiry_ledger + 1),
            &recovery,
            &Address::generate(&env),
        );
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #13)")]
    fn test_double_payment_for_same_asset_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
        );
        client.record_payment(&100, &asset);
        client.record_payment(&50, &asset);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #11)")]
    fn test_sweep_after_expiry_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
        );
        client.record_payment(&100, &asset);

        env.ledger().set_sequence_number(expiry_ledger);

        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);
    }

    #[test]
    fn test_expire_routes_funds_to_recovery_address() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1;

        client.initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
        );
        client.record_payment(&100, &asset);

        env.ledger().set_sequence_number(expiry_ledger);
        client.expire();

        let info = client.get_info();
        assert_eq!(info.status, AccountStatus::Expired);
        assert_eq!(info.swept_to, Some(recovery));
        assert_eq!(client.get_reserve_remaining(), 0);
        assert!(client.is_reserve_reclaimed());
        assert_eq!(client.get_reserve_reclaim_event_count(), 1);
    }

    #[test]
    fn test_initialize_requires_creator_authorization() {
        let env = Env::default();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        let result = client.try_initialize(
            &creator,
            &expiry_ledger,
            &recovery,
            &Address::generate(&env),
        );
        println!("initialize auth result: {:?}", result);

        assert!(matches!(result, Err(Err(InvokeError::Abort))));
    }

    #[test]
    fn test_creator_and_recovery_address_can_be_different() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1;

        // Initialize with different creator and recovery_address
        client.initialize(&creator, &expiry_ledger, &recovery, &controller);
        client.record_payment(&100, &asset);

        // Verify both are stored independently
        let info = client.get_info();
        assert_eq!(info.creator, creator);
        assert_eq!(info.recovery_address, recovery);
        assert_ne!(info.creator, info.recovery_address);

        // Advance to expiry
        env.ledger().set_sequence_number(expiry_ledger);

        // Expire should route funds to recovery_address, not creator
        client.expire();

        let info_after = client.get_info();
        assert_eq!(info_after.status, AccountStatus::Expired);
        assert_eq!(info_after.swept_to, Some(recovery));
        assert_ne!(info_after.swept_to, Some(creator));
    }

    #[test]
    fn test_recovery_address_can_trigger_expire_without_being_creator() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let controller = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1;

        // Initialize with different creator and recovery_address
        client.initialize(&creator, &expiry_ledger, &recovery, &controller);
        client.record_payment(&100, &asset);

        // Advance to expiry
        env.ledger().set_sequence_number(expiry_ledger);

        // expire() does not require authorization - anyone can call it after expiry
        // This verifies recovery_address can trigger recovery without being the creator
        client.expire();

        let info = client.get_info();
        assert_eq!(info.status, AccountStatus::Expired);
        assert_eq!(info.swept_to, Some(recovery));
    }
}
