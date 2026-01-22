#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};
use sweep_controller::{SweepController, SweepControllerClient};

#[test]
fn test_execute_sweep() {
    let env = Env::default();
    env.mock_all_auths();

    // Deploy ephemeral account
    let ephemeral_id = env.register_contract(None, ephemeral_account::EphemeralAccountContract);
    let ephemeral_client =
        ephemeral_account::EphemeralAccountContractClient::new(&env, &ephemeral_id);

    // Deploy sweep controller
    let controller_id = env.register_contract(None, SweepController);
    let controller_client = SweepControllerClient::new(&env, &controller_id);

    // Setup
    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let destination = Address::generate(&env);
    let asset = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1000;

    // Initialize ephemeral account
    ephemeral_client.initialize(&creator, &expiry, &recovery);

    // Record payment
    ephemeral_client.record_payment(&100, &asset);

    // Check can sweep
    assert!(controller_client.can_sweep(&ephemeral_id));

    // Execute sweep
    let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
    controller_client.execute_sweep(&ephemeral_id, &destination, &auth_sig);

    // Verify account status changed
    let status = ephemeral_client.get_status();
    assert_eq!(status, ephemeral_account::AccountStatus::Swept);
}

#[test]
#[should_panic]
fn test_sweep_without_payment() {
    let env = Env::default();
    env.mock_all_auths();

    let ephemeral_id = env.register_contract(None, ephemeral_account::EphemeralAccountContract);
    let ephemeral_client =
        ephemeral_account::EphemeralAccountContractClient::new(&env, &ephemeral_id);

    let controller_id = env.register_contract(None, SweepController);
    let controller_client = SweepControllerClient::new(&env, &controller_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let destination = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1000;

    // Initialize but don't record payment
    ephemeral_client.initialize(&creator, &expiry, &recovery);

    // Should panic - no payment received
    let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
    controller_client.execute_sweep(&ephemeral_id, &destination, &auth_sig);
}

#[test]
fn test_can_sweep() {
    let env = Env::default();
    env.mock_all_auths();

    let ephemeral_id = env.register_contract(None, ephemeral_account::EphemeralAccountContract);
    let ephemeral_client =
        ephemeral_account::EphemeralAccountContractClient::new(&env, &ephemeral_id);

    let controller_id = env.register_contract(None, SweepController);
    let controller_client = SweepControllerClient::new(&env, &controller_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let asset = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1000;

    // Should return false before initialization
    assert!(!controller_client.can_sweep(&ephemeral_id));

    // Initialize
    ephemeral_client.initialize(&creator, &expiry, &recovery);

    // Should return false without payment
    assert!(!controller_client.can_sweep(&ephemeral_id));

    // Record payment
    ephemeral_client.record_payment(&100, &asset);

    // Should return true after payment
    assert!(controller_client.can_sweep(&ephemeral_id));
}
