#![cfg(test)]
extern crate std;

use crate::{
    AccountStatus, EphemeralAccountContract, EphemeralAccountContractClient, Error,
    ReserveReclaimed,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, BytesN, Env,
};

const BASE_RESERVE_STROOPS: i128 = 1_000_000_000;

/// Build a test `Env` with mock auth enabled and the standalone network
/// passphrase, matching what `initialize` enforces via `require_network`.
fn test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger()
        .set_network_id(bridgelet_shared::passphrase::standalone_network_id(&env));
    env
}

fn latest_reserve_event(client: &EphemeralAccountContractClient) -> ReserveReclaimed {
    client
        .get_last_reserve_event()
        .expect("reserve event was not emitted")
}

#[test]
fn test_initialize() {
    let env = test_env();

    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let controller = Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1000;

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &controller,
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );

    assert_eq!(client.get_status(), AccountStatus::Active);
    assert!(!client.is_expired());
    assert_eq!(client.get_reserve_remaining(), BASE_RESERVE_STROOPS);
    assert_eq!(client.get_reserve_available(), BASE_RESERVE_STROOPS);
    assert!(!client.is_reserve_reclaimed());
}

#[test]
fn test_record_payment() {
    let env = test_env();

    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let controller = Address::generate(&env);
    let asset = Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1000;

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &controller,
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );
    client.record_payment(&100, &asset);

    assert_eq!(client.get_status(), AccountStatus::PaymentReceived);
}

#[test]
fn test_multiple_payments() {
    let env = test_env();

    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let controller = Address::generate(&env);
    let asset1 = Address::generate(&env);
    let asset2 = Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1000;

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &controller,
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );

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
    let env = test_env();

    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let controller = Address::generate(&env);
    let asset = Address::generate(&env);
    let destination = Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1000;

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &controller,
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );
    client.record_payment(&100, &asset);

    client.sweep_claim(&destination);

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
    let env = test_env();
    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let controller = Address::generate(&env);
    let asset = Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1000;

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &controller,
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );
    client.record_payment(&100, &asset);
    let result = client.try_record_payment(&50, &asset);

    assert!(matches!(result, Err(Ok(Error::DuplicateAsset))));
}

#[test]
fn test_too_many_assets_returns_expected_error_code() {
    let env = test_env();
    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let controller = Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1000;

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &controller,
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );

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
    let env = test_env();
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
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );
    let result = client.try_record_payment(&0, &asset);

    assert!(matches!(result, Err(Ok(Error::InvalidAmount))));
}

#[test]
fn test_expire_returns_not_expired_error() {
    let env = test_env();
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
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );

    let result = client.try_expire();

    assert!(matches!(result, Err(Ok(Error::NotExpired))));
}

#[test]
fn test_sweep_returns_already_swept_error() {
    let env = test_env();
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
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );
    client.record_payment(&100, &asset);

    client.sweep_claim(&destination);
    let replay_result = client.try_sweep_claim(&destination);

    assert!(matches!(replay_result, Err(Ok(Error::AlreadySwept))));
}

#[test]
fn test_sweep_claim_authorized_controller_succeeds() {
    let env = test_env();
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
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );
    client.record_payment(&100, &asset);

    client.sweep_claim(&destination);

    let info = client.get_info();
    assert_eq!(info.status, AccountStatus::Swept);
    assert_eq!(info.swept_to, Some(destination));
    assert!(client.is_reserve_reclaimed());
}

#[test]
#[should_panic(expected = "Error(Contract, #1010)")]
fn test_sweep_after_expiry_is_rejected() {
    let env = test_env();

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
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );
    client.record_payment(&100, &asset);

    env.ledger().set_sequence_number(expiry_ledger);
    client.sweep_claim(&destination);
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
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );

    assert!(result.is_err());
}

// --- expire() tests (Issue #404: permissionless expiry) ---

#[test]
fn test_expire_succeeds_after_expiry_ledger() {
    let env = test_env();
    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1;

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &Address::generate(&env),
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );

    // Move past expiry
    env.ledger().set_sequence_number(expiry_ledger);

    client.expire();

    assert_eq!(client.get_status(), AccountStatus::Expired);
}

#[test]
fn test_expire_is_permissionless() {
    let env = test_env();
    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1;
    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &Address::generate(&env),
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );

    // Move past expiry
    env.ledger().set_sequence_number(expiry_ledger);

    // Any address can call expire — no auth required
    client.expire();

    assert_eq!(client.get_status(), AccountStatus::Expired);
    // Funds routed to recovery address
    let info = client.get_info();
    assert_eq!(info.swept_to, Some(recovery));
}

#[test]
fn test_expire_returns_invalid_status_when_already_swept() {
    let env = test_env();
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
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );
    client.record_payment(&100, &asset);
    client.sweep_claim(&destination);

    let result = client.try_expire();
    assert!(matches!(result, Err(Ok(Error::InvalidStatus))));
}

#[test]
fn test_expire_returns_invalid_status_when_already_expired() {
    let env = test_env();
    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1;

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &Address::generate(&env),
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );

    env.ledger().set_sequence_number(expiry_ledger);
    client.expire();

    // Second expire should fail
    let result = client.try_expire();
    assert!(matches!(result, Err(Ok(Error::InvalidStatus))));
}

// --- recover() tests (Issue #404: gated by creator/recovery_address) ---

#[test]
fn test_recover_succeeds_for_creator() {
    let env = test_env();
    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1;

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &Address::generate(&env),
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );

    env.ledger().set_sequence_number(expiry_ledger);

    client.recover(&creator);

    assert_eq!(client.get_status(), AccountStatus::Expired);
    assert_eq!(client.get_info().swept_to, Some(recovery));
}

#[test]
fn test_recover_succeeds_for_recovery_address() {
    let env = test_env();
    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1;

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &Address::generate(&env),
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );

    env.ledger().set_sequence_number(expiry_ledger);

    client.recover(&recovery);

    assert_eq!(client.get_status(), AccountStatus::Expired);
    assert_eq!(client.get_info().swept_to, Some(recovery));
}

#[test]
fn test_recover_rejects_unauthorized_caller() {
    let env = test_env();
    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1;
    let random_caller = Address::generate(&env);

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &Address::generate(&env),
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );

    env.ledger().set_sequence_number(expiry_ledger);

    let result = client.try_recover(&random_caller);
    assert!(matches!(result, Err(Ok(Error::Unauthorized))));
}

#[test]
fn test_recover_returns_not_expired_before_expiry() {
    let env = test_env();
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
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );

    let result = client.try_recover(&creator);
    assert!(matches!(result, Err(Ok(Error::NotExpired))));
}

#[test]
fn test_recover_returns_invalid_status_when_already_expired() {
    let env = test_env();
    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1;

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &Address::generate(&env),
        &BytesN::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &None::<Address>,
    );

    env.ledger().set_sequence_number(expiry_ledger);
    client.expire();

    // recover() should fail since already expired
    let result = client.try_recover(&creator);
    assert!(matches!(result, Err(Ok(Error::InvalidStatus))));
}

// --- Issue #405: cross-contract reserve fetch from ReserveContract ---

const CUSTOM_RESERVE: i128 = 500_000_000; // 50 XLM

/// Helper: deploy and initialize a ReserveContract with the given admin.
fn setup_reserve_contract(
    env: &Env,
) -> (
    soroban_sdk::Address,
    reserve_contract::ReserveContractClient<'_>,
) {
    let reserve_id = env.register(reserve_contract::ReserveContract, ());
    let reserve_client = reserve_contract::ReserveContractClient::new(env, &reserve_id);
    let admin = soroban_sdk::Address::generate(env);
    reserve_client.initialize(&admin);
    (admin, reserve_client)
}

/// When `reserve_contract` is `Some(addr)` and the remote contract has a
/// configured value, the ephemeral account must use that dynamic value
/// instead of the compile-time constant.
#[test]
fn test_initialize_uses_dynamic_reserve_from_contract() {
    let env = test_env();
    let (_admin, reserve_client) = setup_reserve_contract(&env);
    reserve_client.set_base_reserve(&CUSTOM_RESERVE);
    let reserve_id = reserve_client.address.clone();

    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = soroban_sdk::Address::generate(&env);
    let recovery = soroban_sdk::Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1000;

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &soroban_sdk::Address::generate(&env),
        &soroban_sdk::BytesN::from_array(&env, &[0u8; 32]),
        &soroban_sdk::Address::generate(&env),
        &Some(reserve_id),
    );

    assert_eq!(client.get_reserve_remaining(), CUSTOM_RESERVE);
    assert_eq!(client.get_reserve_available(), CUSTOM_RESERVE);
}

/// When `reserve_contract` is `Some(addr)` but the remote contract has
/// not yet called `set_base_reserve`, the ephemeral account falls back
/// to the compile-time default.
#[test]
fn test_initialize_falls_back_when_reserve_not_set() {
    let env = test_env();

    let (_admin, reserve_client) = setup_reserve_contract(&env);
    // Do NOT call set_base_reserve — the value is unset.

    let reserve_id = reserve_client.address.clone();

    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = soroban_sdk::Address::generate(&env);
    let recovery = soroban_sdk::Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1000;

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &soroban_sdk::Address::generate(&env),
        &soroban_sdk::BytesN::from_array(&env, &[0u8; 32]),
        &soroban_sdk::Address::generate(&env),
        &Some(reserve_id),
    );

    // Should fall back to the compile-time constant
    assert_eq!(client.get_reserve_remaining(), BASE_RESERVE_STROOPS);
    assert_eq!(client.get_reserve_available(), BASE_RESERVE_STROOPS);
}

/// When `reserve_contract` is `None`, the compile-time constant is used
/// (same as before Issue #405).
#[test]
fn test_initialize_uses_constant_when_no_reserve_contract() {
    let env = test_env();

    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = soroban_sdk::Address::generate(&env);
    let recovery = soroban_sdk::Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1000;

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &soroban_sdk::Address::generate(&env),
        &soroban_sdk::BytesN::from_array(&env, &[0u8; 32]),
        &soroban_sdk::Address::generate(&env),
        &None::<soroban_sdk::Address>,
    );

    assert_eq!(client.get_reserve_remaining(), BASE_RESERVE_STROOPS);
    assert_eq!(client.get_reserve_available(), BASE_RESERVE_STROOPS);
}

/// Sweep reclaims the dynamic reserve amount, not the compile-time constant.
#[test]
fn test_sweep_reclaims_dynamic_reserve() {
    let env = test_env();

    let (_admin, reserve_client) = setup_reserve_contract(&env);
    reserve_client.set_base_reserve(&CUSTOM_RESERVE);
    let reserve_id = reserve_client.address.clone();

    let contract_id = env.register(EphemeralAccountContract, ());
    let client = EphemeralAccountContractClient::new(&env, &contract_id);

    let creator = soroban_sdk::Address::generate(&env);
    let recovery = soroban_sdk::Address::generate(&env);
    let asset = soroban_sdk::Address::generate(&env);
    let destination = soroban_sdk::Address::generate(&env);
    let expiry_ledger = env.ledger().sequence() + 1000;

    client.initialize(
        &creator,
        &expiry_ledger,
        &recovery,
        &soroban_sdk::Address::generate(&env),
        &soroban_sdk::BytesN::from_array(&env, &[0u8; 32]),
        &soroban_sdk::Address::generate(&env),
        &Some(reserve_id),
    );
    client.record_payment(&100, &asset);

    client.sweep_claim(&destination);

    let reserve_event = latest_reserve_event(&client);
    assert_eq!(reserve_event.amount, CUSTOM_RESERVE);
    assert_eq!(client.get_reserve_remaining(), 0);
    assert!(client.is_reserve_reclaimed());
}
