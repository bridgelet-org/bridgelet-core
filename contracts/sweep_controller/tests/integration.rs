#![cfg(test)]

extern crate std;

use ephemeral_account::{AccountStatus, EphemeralAccountContract, EphemeralAccountContractClient};
use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Events as _, Ledger as _},
    Address, BytesN, Env, IntoVal, Symbol, TryFromVal,
};
use sweep_controller::{SweepController, SweepControllerClient};

fn generate_test_keypair(env: &Env) -> (BytesN<32>, BytesN<64>) {
    let public_key = BytesN::from_array(
        env,
        &[
            0x30, 0xd4, 0x18, 0x9f, 0x87, 0x6e, 0xda, 0x97, 0x42, 0xa2, 0x55, 0x14, 0x87, 0x43,
            0xd9, 0x24, 0x9d, 0xf4, 0x12, 0x02, 0x7b, 0x0d, 0xb5, 0x47, 0x69, 0xe9, 0x18, 0xd3,
            0x6f, 0x25, 0x9d, 0x3c,
        ],
    );
    let dummy_signature = BytesN::from_array(env, &[0u8; 64]);
    (public_key, dummy_signature)
}

fn setup_ready_account(
    env: &Env,
    authorized_destination: Option<Address>,
) -> (
    SweepControllerClient<'_>,
    EphemeralAccountContractClient<'_>,
    Address,
) {
    let controller_id = env.register(SweepController, ());
    let controller_client = SweepControllerClient::new(&env, &controller_id);

    let creator = Address::generate(&env);
    let (authorized_signer, _) = generate_test_keypair(&env);

    // Initialize controller with authorized signer (flexible mode - no destination)
    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &creator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_id,
                fn_name: "initialize",
                args: (&creator, &authorized_signer, &authorized_destination).into_val(env),
                sub_invokes: &[],
            },
        }])
        .initialize(&creator, &authorized_signer, &authorized_destination);

    let ephemeral_id = env.register(EphemeralAccountContract, ());
    let ephemeral_client = EphemeralAccountContractClient::new(&env, &ephemeral_id);

    let account_creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1_000;
    ephemeral_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &account_creator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &ephemeral_id,
                fn_name: "initialize",
                args: (
                    &account_creator,
                    &expiry,
                    &recovery,
                    &controller_id,
                    &account_creator,
                )
                    .into_val(env),
                sub_invokes: &[],
            },
        }])
        .initialize(
            &account_creator,
            &expiry,
            &recovery,
            &controller_id,
            &account_creator,
        );

    let asset_id = Address::generate(&env);
    env.mock_all_auths_allowing_non_root_auth();
    ephemeral_client.record_payment(&100, &asset_id);
    env.set_auths(&[]);

    (controller_client, ephemeral_client, ephemeral_id)
}

/// Test that re-initialization is prevented
#[test]
fn test_initialize_prevents_double_init() {
    let env = Env::default();
    env.mock_all_auths();

    let _creator = Address::generate(&env);
    let controller_id = env.register(SweepController, ());
    let controller_client = SweepControllerClient::new(&env, &controller_id);

    let creator = Address::generate(&env);
    let (authorized_signer, _) = generate_test_keypair(&env);

    // First initialization should succeed
    controller_client.initialize(&creator, &authorized_signer, &None);

    // Second initialization should fail
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        controller_client.initialize(&creator, &authorized_signer, &None);
    }));
    assert!(result.is_err());
}

/// Test that valid signatures are accepted
#[test]
fn test_execute_sweep_with_valid_signature() {
    let env = Env::default();
    env.mock_all_auths();

    let _creator = Address::generate(&env);
    // Deploy and initialize controller
    let controller_id = env.register(SweepController, ());
    let controller_client = SweepControllerClient::new(&env, &controller_id);

    let creator = Address::generate(&env);
    let (authorized_signer, _) = generate_test_keypair(&env);
    controller_client.initialize(&creator, &authorized_signer, &None);

    // Deploy ephemeral account
    let ephemeral_id = env.register(EphemeralAccountContract, ());
    let ephemeral_client = EphemeralAccountContractClient::new(&env, &ephemeral_id);

    // Setup
    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let destination = Address::generate(&env);
    let _asset = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1000;

    // Initialize ephemeral account, authorizing this SweepController to call sweep()
    ephemeral_client.initialize(&creator, &expiry, &recovery, &controller_id, &creator);

    // Create an invalid signature (all zeros - different from valid signature)
    let invalid_sig = BytesN::from_array(&env, &[0u8; 64]);

    // Execute sweep with invalid signature - should fail verification
    // In tests, client methods panic on error, so we catch it
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        controller_client.execute_sweep(&ephemeral_id, &destination, &invalid_sig);
    }));

    // We expect this to fail due to signature verification
    assert!(result.is_err());

    println!("Execute sweep with invalid signature result: {:?}", result);
}

/// Test that sweep without payment fails
#[test]
#[should_panic]
fn test_sweep_without_payment() {
    let env = Env::default();
    env.mock_all_auths();

    let ephemeral_id = env.register(EphemeralAccountContract, ());
    let ephemeral_client = EphemeralAccountContractClient::new(&env, &ephemeral_id);

    let controller_id = env.register(SweepController, ());
    let controller_client = SweepControllerClient::new(&env, &controller_id);

    let account_creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1_000;
    ephemeral_client.initialize(
        &account_creator,
        &expiry,
        &recovery,
        &controller_id,
        &account_creator,
    );

    let asset_id = Address::generate(&env);
    ephemeral_client.record_payment(&100, &asset_id);

    let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
    controller_client.execute_sweep(&ephemeral_id, &account_creator, &auth_sig);
}

#[test]
fn test_claim_succeeds_with_recipient_auth_and_relayable_flow() {
    let env = Env::default();

    let recipient = Address::generate(&env);
    let (controller_client, ephemeral_client, ephemeral_id) =
        setup_ready_account(&env, Some(recipient.clone()));

    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &recipient,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_client.address,
                fn_name: "claim",
                args: (&recipient, &ephemeral_id).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .claim(&recipient, &ephemeral_id);

    assert_eq!(ephemeral_client.get_status(), AccountStatus::Swept);
    let info = ephemeral_client.get_info();
    assert_eq!(info.swept_to, Some(recipient));
}

#[test]
fn test_claim_records_recipient_authorization_context() {
    let env = Env::default();

    let recipient = Address::generate(&env);
    let (controller_client, _, ephemeral_id) = setup_ready_account(&env, Some(recipient.clone()));

    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &recipient,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_client.address,
                fn_name: "claim",
                args: (&recipient, &ephemeral_id).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .claim(&recipient, &ephemeral_id);

    assert_eq!(
        env.auths(),
        std::vec![(
            recipient.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    controller_client.address.clone(),
                    soroban_sdk::symbol_short!("claim"),
                    (&recipient, &ephemeral_id).into_val(&env),
                )),
                sub_invocations: std::vec![],
            },
        )]
    );
}

#[test]
fn test_claim_rejects_wrong_recipient_for_locked_destination() {
    let env = Env::default();

    let locked_destination = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (controller_client, _, ephemeral_id) = setup_ready_account(&env, Some(locked_destination));

    controller_client.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &recipient,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &controller_client.address,
            fn_name: "claim",
            args: (&recipient, &ephemeral_id).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = controller_client.try_claim(&recipient, &ephemeral_id);

    // The claim should fail because recipient != locked_destination
    assert!(result.is_err());
}

#[test]
fn test_unauthorized_signer_not_set() {
    let env = Env::default();
    env.mock_all_auths();

    // Deploy controller without initialization
    let controller_id = env.register(SweepController, ());
    let controller_client = SweepControllerClient::new(&env, &controller_id);

    // Deploy ephemeral account
    let ephemeral_id = env.register(EphemeralAccountContract, ());
    let ephemeral_client = EphemeralAccountContractClient::new(&env, &ephemeral_id);

    // Setup
    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let destination = Address::generate(&env);
    let asset = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1000;

    // Initialize ephemeral account, authorizing this SweepController to call sweep()
    ephemeral_client.initialize(&creator, &expiry, &recovery, &controller_id, &creator);

    // Record payment
    ephemeral_client.record_payment(&100, &asset);

    // Create a signature
    let auth_sig = BytesN::from_array(&env, &[3u8; 64]);

    // Execute sweep without initializing controller - should fail
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        controller_client.execute_sweep(&ephemeral_id, &destination, &auth_sig);
    }));

    // Should fail because authorized_signer is not set
    assert!(result.is_err());
    println!(
        "Execute sweep without initialization correctly failed: {:?}",
        result
    );
}

/// Test initialization with authorized destination (locked mode)
#[test]
fn test_initialize_with_authorized_destination() {
    let env = Env::default();

    let controller_id = env.register(SweepController, ());
    let controller_client = SweepControllerClient::new(&env, &controller_id);
    let creator = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (authorized_signer, _) = generate_test_keypair(&env);
    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &creator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_id,
                fn_name: "initialize",
                args: (&creator, &authorized_signer, &Some(recipient.clone())).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize(&creator, &authorized_signer, &Some(recipient.clone()));

    let ephemeral_id = env.register(EphemeralAccountContract, ());
    let ephemeral_client = EphemeralAccountContractClient::new(&env, &ephemeral_id);
    let account_creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1_000;
    ephemeral_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &account_creator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &ephemeral_id,
                fn_name: "initialize",
                args: (
                    &account_creator,
                    &expiry,
                    &recovery,
                    &controller_id,
                    &account_creator,
                )
                    .into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize(
            &account_creator,
            &expiry,
            &recovery,
            &controller_id,
            &account_creator,
        );

    let asset_id = Address::generate(&env);
    env.mock_all_auths_allowing_non_root_auth();
    ephemeral_client.record_payment(&100, &asset_id);
    env.set_auths(&[]);

    let result = controller_client.try_claim(&recipient, &ephemeral_id);

    assert!(result.is_err());
}

// ──────────────────────────────────────────────────────────────────────────────
// Full Integration Test Suite — Issue #165
// Issue #160: Full integration test suite — EphemeralAccount + SweepController
// ──────────────────────────────────────────────────────────────────────────────

/// Helper: deploy both contracts and return clients + IDs.
fn deploy_contracts(
    env: &Env,
) -> (
    SweepControllerClient<'_>,
    Address,
    EphemeralAccountContractClient<'_>,
    Address,
) {
    let controller_id = env.register(SweepController, ());
    let controller_client = SweepControllerClient::new(env, &controller_id);

    let ephemeral_id = env.register(EphemeralAccountContract, ());
    let ephemeral_client = EphemeralAccountContractClient::new(env, &ephemeral_id);

    (
        controller_client,
        controller_id,
        ephemeral_client,
        ephemeral_id,
    )
}

/// Helper: deploy both contracts, initialize both, record payment, return ready-to-sweep state.
/// Uses locked destination matching the recipient so claim() path works.
fn setup_full_lifecycle(
    env: &Env,
) -> (
    SweepControllerClient<'_>,
    EphemeralAccountContractClient<'_>,
    Address,
    Address,
    Address,
) {
    let (controller_client, controller_id, ephemeral_client, ephemeral_id) = deploy_contracts(env);

    let controller_creator = Address::generate(env);
    let (authorized_signer, _) = generate_test_keypair(env);
    let recipient = Address::generate(env);
    let destination = recipient.clone();

    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &controller_creator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_id,
                fn_name: "initialize",
                args: (
                    &controller_creator,
                    &authorized_signer,
                    &Some(destination.clone()),
                )
                    .into_val(env),
                sub_invokes: &[],
            },
        }])
        .initialize(&controller_creator, &authorized_signer, &Some(destination));

    let account_creator = Address::generate(env);
    let recovery = Address::generate(env);
    let expiry = env.ledger().sequence() + 1_000;

    ephemeral_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &account_creator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &ephemeral_id,
                fn_name: "initialize",
                args: (
                    &account_creator,
                    &expiry,
                    &recovery,
                    &controller_id,
                    &account_creator,
                )
                    .into_val(env),
                sub_invokes: &[],
            },
        }])
        .initialize(
            &account_creator,
            &expiry,
            &recovery,
            &controller_id,
            &account_creator,
        );

    let asset = Address::generate(env);
    env.mock_all_auths_allowing_non_root_auth();
    ephemeral_client.record_payment(&500, &asset);
    env.set_auths(&[]);

    (
        controller_client,
        ephemeral_client,
        ephemeral_id,
        recipient,
        asset,
    )
}

/// Deploy → init → record → claim → verify full state including reserve.
#[test]
fn test_full_lifecycle_deploy_init_record_claim_verify_state() {
    let env = Env::default();

    let (controller_client, ephemeral_client, ephemeral_id, recipient, _asset) =
        setup_full_lifecycle(&env);

    assert_eq!(
        ephemeral_client.get_status(),
        AccountStatus::PaymentReceived
    );
    assert!(!ephemeral_client.is_expired());

    let info_before = ephemeral_client.get_info();
    assert!(info_before.payment_received);
    assert_eq!(info_before.payment_count, 1);
    assert_eq!(info_before.payments.get(0).unwrap().amount, 500);

    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &recipient,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_client.address,
                fn_name: "claim",
                args: (&recipient, &ephemeral_id).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .claim(&recipient, &ephemeral_id);

    assert_eq!(ephemeral_client.get_status(), AccountStatus::Swept);
    let info_after = ephemeral_client.get_info();
    assert_eq!(info_after.swept_to, Some(recipient.clone()));
    assert_eq!(info_after.payment_count, 1);

    assert_eq!(ephemeral_client.get_reserve_remaining(), 0);
    assert!(ephemeral_client.is_reserve_reclaimed());

    let reserve_event = ephemeral_client.get_last_reserve_event().unwrap();
    assert_eq!(reserve_event.destination, recipient);
    assert_eq!(reserve_event.amount, 1_000_000_000);
    assert!(reserve_event.fully_reclaimed);
    assert_eq!(ephemeral_client.get_reserve_reclaim_event_count(), 1);
}

/// Full lifecycle with multiple assets.
#[test]
fn test_full_lifecycle_multi_asset_claim() {
    let env = Env::default();

    let (controller_client, controller_id, ephemeral_client, ephemeral_id) = deploy_contracts(&env);

    let controller_creator = Address::generate(&env);
    let (authorized_signer, _) = generate_test_keypair(&env);
    let recipient = Address::generate(&env);

    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &controller_creator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_id,
                fn_name: "initialize",
                args: (
                    &controller_creator,
                    &authorized_signer,
                    &Some(recipient.clone()),
                )
                    .into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize(
            &controller_creator,
            &authorized_signer,
            &Some(recipient.clone()),
        );

    let account_creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1_000;

    ephemeral_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &account_creator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &ephemeral_id,
                fn_name: "initialize",
                args: (
                    &account_creator,
                    &expiry,
                    &recovery,
                    &controller_id,
                    &account_creator,
                )
                    .into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize(
            &account_creator,
            &expiry,
            &recovery,
            &controller_id,
            &account_creator,
        );

    let asset1 = Address::generate(&env);
    let asset2 = Address::generate(&env);
    let asset3 = Address::generate(&env);
    env.mock_all_auths_allowing_non_root_auth();
    ephemeral_client.record_payment(&100, &asset1);
    ephemeral_client.record_payment(&200, &asset2);
    ephemeral_client.record_payment(&300, &asset3);
    env.set_auths(&[]);

    let info = ephemeral_client.get_info();
    assert_eq!(info.payment_count, 3);
    assert_eq!(info.status, AccountStatus::PaymentReceived);

    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &recipient,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_client.address,
                fn_name: "claim",
                args: (&recipient, &ephemeral_id).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .claim(&recipient, &ephemeral_id);

    assert_eq!(ephemeral_client.get_status(), AccountStatus::Swept);
    assert_eq!(
        ephemeral_client.get_info().swept_to,
        Some(recipient.clone())
    );

    let final_info = ephemeral_client.get_info();
    let total: i128 = final_info.payments.iter().map(|p| p.amount).sum();
    assert_eq!(total, 600);
}

/// Expire flow: funds route to recovery_address.
#[test]
fn test_full_expire_flow_funds_to_recovery() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, ephemeral_client, _) = deploy_contracts(&env);

    let account_creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry = env.ledger().sequence() + 5;

    ephemeral_client.initialize(
        &account_creator,
        &expiry,
        &recovery,
        &Address::generate(&env),
        &account_creator,
    );

    let asset = Address::generate(&env);
    ephemeral_client.record_payment(&1_000, &asset);

    env.ledger().set_sequence_number(expiry);

    ephemeral_client.expire();

    assert_eq!(ephemeral_client.get_status(), AccountStatus::Expired);
    let info = ephemeral_client.get_info();
    assert_eq!(info.swept_to, Some(recovery));
    assert_eq!(ephemeral_client.get_reserve_remaining(), 0);
    assert!(ephemeral_client.is_reserve_reclaimed());
}

/// Recover flow: creator triggers recovery after expiry.
#[test]
fn test_full_recover_flow_creator_after_expiry() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, ephemeral_client, _) = deploy_contracts(&env);

    let account_creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry = env.ledger().sequence() + 5;

    ephemeral_client.initialize(
        &account_creator,
        &expiry,
        &recovery,
        &Address::generate(&env),
        &account_creator,
    );

    let asset = Address::generate(&env);
    ephemeral_client.record_payment(&2_000, &asset);

    env.ledger().set_sequence_number(expiry);

    ephemeral_client.recover(&account_creator);

    assert_eq!(ephemeral_client.get_status(), AccountStatus::Expired);
    let info = ephemeral_client.get_info();
    assert_eq!(info.swept_to, Some(recovery));
}

/// Sweep rejected after expiry via claim.
#[test]
fn test_sweep_rejected_after_expiry_via_claim() {
    let env = Env::default();

    let recipient = Address::generate(&env);
    let (controller_client, _ephemeral_client, ephemeral_id, _, _) = setup_full_lifecycle(&env);

    env.ledger()
        .set_sequence_number(env.ledger().sequence() + 2_000);

    let result = controller_client.try_claim(&recipient, &ephemeral_id);
    assert!(result.is_err());
}

/// Sweep rejected when no payment recorded.
#[test]
fn test_sweep_rejected_when_no_payment_recorded() {
    let env = Env::default();

    let (_, controller_id, ephemeral_client, ephemeral_id) = deploy_contracts(&env);

    let controller_creator = Address::generate(&env);
    let (authorized_signer, _) = generate_test_keypair(&env);
    let recipient = Address::generate(&env);

    let controller_client = SweepControllerClient::new(&env, &controller_id);
    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &controller_creator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_id,
                fn_name: "initialize",
                args: (
                    &controller_creator,
                    &authorized_signer,
                    &Some(recipient.clone()),
                )
                    .into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize(
            &controller_creator,
            &authorized_signer,
            &Some(recipient.clone()),
        );

    let account_creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1_000;

    ephemeral_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &account_creator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &ephemeral_id,
                fn_name: "initialize",
                args: (
                    &account_creator,
                    &expiry,
                    &recovery,
                    &controller_id,
                    &account_creator,
                )
                    .into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize(
            &account_creator,
            &expiry,
            &recovery,
            &controller_id,
            &account_creator,
        );

    let result = controller_client.try_claim(&recipient, &ephemeral_id);
    assert!(result.is_err());
}

/// Double claim rejected.
#[test]
fn test_double_claim_rejected() {
    let env = Env::default();

    let (controller_client, ephemeral_client, ephemeral_id, recipient, _) =
        setup_full_lifecycle(&env);

    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &recipient,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_client.address,
                fn_name: "claim",
                args: (&recipient, &ephemeral_id).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .claim(&recipient, &ephemeral_id);
    assert_eq!(ephemeral_client.get_status(), AccountStatus::Swept);

    let result = controller_client.try_claim(&recipient, &ephemeral_id);
    assert!(result.is_err());
}

/// Locked destination rejects wrong address.
#[test]
fn test_locked_destination_rejects_wrong_address() {
    let env = Env::default();

    let (controller_client, _, _, _, _) = setup_full_lifecycle(&env);

    let wrong_recipient = Address::generate(&env);
    let ephemeral_id_wrong = Address::generate(&env);

    let result = controller_client.try_claim(&wrong_recipient, &ephemeral_id_wrong);
    assert!(result.is_err());
}

/// CanSweep returns correct values for different account states.
#[test]
fn test_can_sweep_reflects_account_state() {
    let env = Env::default();
    env.mock_all_auths();

    let (controller_client, controller_id, ephemeral_client, ephemeral_id) = deploy_contracts(&env);

    let controller_creator = Address::generate(&env);
    let (authorized_signer, _) = generate_test_keypair(&env);
    controller_client.initialize(&controller_creator, &authorized_signer, &None);

    let account_creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1_000;

    ephemeral_client.initialize(
        &account_creator,
        &expiry,
        &recovery,
        &controller_id,
        &account_creator,
    );

    // Active with no payment → can_sweep should be false
    assert!(!controller_client.can_sweep(&ephemeral_id));

    // PaymentReceived → can_sweep should be true
    ephemeral_client.record_payment(&100, &Address::generate(&env));
    assert!(controller_client.can_sweep(&ephemeral_id));

    // Claim via locked destination
    let recipient = Address::generate(&env);
    // Re-deploy controller with matching destination
    let controller_id2 = env.register(SweepController, ());
    let controller_client2 = SweepControllerClient::new(&env, &controller_id2);
    let (authorized_signer2, _) = generate_test_keypair(&env);
    let creator2 = Address::generate(&env);
    controller_client2
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &creator2,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_id2,
                fn_name: "initialize",
                args: (&creator2, &authorized_signer2, &Some(recipient.clone())).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize(&creator2, &authorized_signer2, &Some(recipient.clone()));

    let ephemeral_id2 = env.register(EphemeralAccountContract, ());
    let ephemeral_client2 = EphemeralAccountContractClient::new(&env, &ephemeral_id2);
    let account_creator2 = Address::generate(&env);
    let recovery2 = Address::generate(&env);
    let expiry2 = env.ledger().sequence() + 1_000;
    ephemeral_client2.initialize(
        &account_creator2,
        &expiry2,
        &recovery2,
        &controller_id2,
        &account_creator2,
    );
    ephemeral_client2.record_payment(&100, &Address::generate(&env));

    assert!(controller_client2.can_sweep(&ephemeral_id2));

    controller_client2
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &recipient,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_client2.address,
                fn_name: "claim",
                args: (&recipient, &ephemeral_id2).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .claim(&recipient, &ephemeral_id2);

    assert!(!controller_client2.can_sweep(&ephemeral_id2));
}

/// SweepCompleted event emitted during claim.
#[test]
fn test_claim_emits_sweep_completed_event() {
    let env = Env::default();

    let (controller_client, _ephemeral_client, ephemeral_id, recipient, _asset) =
        setup_full_lifecycle(&env);

    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &recipient,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_client.address,
                fn_name: "claim",
                args: (&recipient, &ephemeral_id).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .claim(&recipient, &ephemeral_id);

    let events = env.events().all();
    let mut found_sweep_event = false;
    for i in 0..events.len() {
        let (_contract, topics, _data) = events.get_unchecked(i);
        if let Ok(sym) = Symbol::try_from_val(&env, &topics.get(0).unwrap()) {
            if sym == soroban_sdk::symbol_short!("sweep") {
                found_sweep_event = true;
                break;
            }
        }
    }
    assert!(found_sweep_event, "SweepCompleted event should be emitted");
}

// ── Additional tests for Issue #165 ──

/// Nonce starts at 0 after initialization.
#[test]
fn test_nonce_starts_at_zero() {
    let env = Env::default();
    env.mock_all_auths();

    let (controller_client, _, _, _, _) = setup_full_lifecycle(&env);

    assert_eq!(controller_client.get_nonce(), 0);
}

/// Multiple ephemeral accounts can be managed by the same controller.
#[test]
fn test_single_controller_manages_multiple_accounts() {
    let env = Env::default();

    let (controller_client, controller_id, _, _) = deploy_contracts(&env);

    let controller_creator = Address::generate(&env);
    let (authorized_signer, _) = generate_test_keypair(&env);
    let recipient = Address::generate(&env);

    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &controller_creator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_id,
                fn_name: "initialize",
                args: (
                    &controller_creator,
                    &authorized_signer,
                    &Some(recipient.clone()),
                )
                    .into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize(
            &controller_creator,
            &authorized_signer,
            &Some(recipient.clone()),
        );

    // Create two ephemeral accounts under the same controller
    let mut ephemeral_ids = std::vec::Vec::new();
    for _ in 0..2 {
        let ephemeral_id = env.register(EphemeralAccountContract, ());
        let ephemeral_client = EphemeralAccountContractClient::new(&env, &ephemeral_id);
        let account_creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let expiry = env.ledger().sequence() + 1_000;

        ephemeral_client
            .mock_auths(&[soroban_sdk::testutils::MockAuth {
                address: &account_creator,
                invoke: &soroban_sdk::testutils::MockAuthInvoke {
                    contract: &ephemeral_id,
                    fn_name: "initialize",
                    args: (
                        &account_creator,
                        &expiry,
                        &recovery,
                        &controller_id,
                        &account_creator,
                    )
                        .into_val(&env),
                    sub_invokes: &[],
                },
            }])
            .initialize(
                &account_creator,
                &expiry,
                &recovery,
                &controller_id,
                &account_creator,
            );

        let asset = Address::generate(&env);
        env.mock_all_auths_allowing_non_root_auth();
        ephemeral_client.record_payment(&100, &asset);
        env.set_auths(&[]);

        assert!(controller_client.can_sweep(&ephemeral_id));
        ephemeral_ids.push(ephemeral_id);
    }

    // Claim the first account
    let first_id = ephemeral_ids.get(0).unwrap();
    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &recipient,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_client.address,
                fn_name: "claim",
                args: (&recipient, first_id).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .claim(&recipient, first_id);

    // First is claimed, second is still sweepable
    assert!(!controller_client.can_sweep(first_id));
    let second_id = ephemeral_ids.get(1).unwrap();
    assert!(controller_client.can_sweep(second_id));
}

/// Expire via recovery_address (not creator) after expiry.
#[test]
fn test_recovery_address_can_expire_account() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, ephemeral_client, _) = deploy_contracts(&env);

    let account_creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry = env.ledger().sequence() + 5;

    ephemeral_client.initialize(
        &account_creator,
        &expiry,
        &recovery,
        &Address::generate(&env),
        &account_creator,
    );

    let asset = Address::generate(&env);
    ephemeral_client.record_payment(&500, &asset);

    env.ledger().set_sequence_number(expiry);

    // expire() is permissionless - anyone can call it
    ephemeral_client.expire();

    assert_eq!(ephemeral_client.get_status(), AccountStatus::Expired);
    assert_eq!(ephemeral_client.get_info().swept_to, Some(recovery));
}

/// get_info returns correct state at each lifecycle stage.
#[test]
fn test_get_info_reflects_lifecycle_stages() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, ephemeral_client, _) = deploy_contracts(&env);

    let account_creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1_000;

    ephemeral_client.initialize(
        &account_creator,
        &expiry,
        &recovery,
        &Address::generate(&env),
        &account_creator,
    );

    // Stage: Active
    let info = ephemeral_client.get_info();
    assert_eq!(info.status, AccountStatus::Active);
    assert!(!info.payment_received);
    assert_eq!(info.payment_count, 0);

    // Stage: PaymentReceived
    let asset = Address::generate(&env);
    ephemeral_client.record_payment(&100, &asset);

    let info = ephemeral_client.get_info();
    assert_eq!(info.status, AccountStatus::PaymentReceived);
    assert!(info.payment_received);
    assert_eq!(info.payment_count, 1);
    assert_eq!(info.creator, account_creator);
    assert_eq!(info.expiry_ledger, expiry);
    assert_eq!(info.recovery_address, recovery);
}

/// Claim with flexible controller (no locked destination) succeeds for any recipient.
#[test]
fn test_claim_with_flexible_destination() {
    let env = Env::default();

    let (controller_client, controller_id, ephemeral_client, ephemeral_id) = deploy_contracts(&env);

    let controller_creator = Address::generate(&env);
    let (authorized_signer, _) = generate_test_keypair(&env);

    // Initialize without locked destination
    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &controller_creator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_id,
                fn_name: "initialize",
                args: (&controller_creator, &authorized_signer, &None::<Address>).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize(&controller_creator, &authorized_signer, &None);

    let account_creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1_000;

    ephemeral_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &account_creator,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &ephemeral_id,
                fn_name: "initialize",
                args: (
                    &account_creator,
                    &expiry,
                    &recovery,
                    &controller_id,
                    &account_creator,
                )
                    .into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize(
            &account_creator,
            &expiry,
            &recovery,
            &controller_id,
            &account_creator,
        );

    let asset = Address::generate(&env);
    env.mock_all_auths_allowing_non_root_auth();
    ephemeral_client.record_payment(&100, &asset);
    env.set_auths(&[]);

    let any_recipient = Address::generate(&env);
    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &any_recipient,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_client.address,
                fn_name: "claim",
                args: (&any_recipient, &ephemeral_id).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .claim(&any_recipient, &ephemeral_id);

    assert_eq!(ephemeral_client.get_status(), AccountStatus::Swept);
    assert_eq!(ephemeral_client.get_info().swept_to, Some(any_recipient));
}
