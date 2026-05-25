#[cfg(test)]
mod test {
    extern crate std;

    use crate::{
        storage, AccountStatus, EphemeralAccountContract, EphemeralAccountContractClient,
        ReserveReclaimed,
    };
    use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

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
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);

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
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);
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
        let asset1 = Address::generate(&env);
        let asset2 = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);

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
        let asset = Address::generate(&env);
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);
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
    #[should_panic(expected = "Error(Contract, #13)")]
    fn test_duplicate_asset() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EphemeralAccountContract, ());
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);
        client.record_payment(&100, &asset);
        client.record_payment(&50, &asset);
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
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);

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
        let destination = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);

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
        let destination = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);
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
        let destination = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);
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
        let destination = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        client.initialize(&creator, &expiry_ledger, &recovery);
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
}
