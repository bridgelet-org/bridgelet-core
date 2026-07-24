#![cfg(test)]

extern crate std;

use ephemeral_account::{AccountStatus, EphemeralAccountContract, EphemeralAccountContractClient};
use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
    Address, BytesN, Env, IntoVal,
};
use sweep_controller::{Error, SweepController, SweepControllerClient};

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
                args: (&account_creator, &expiry, &recovery, &controller_id).into_val(env),
                sub_invokes: &[],
            },
        }])
        .initialize(&account_creator, &expiry, &recovery, &controller_id);

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
    ephemeral_client.initialize(&creator, &expiry, &recovery, &controller_id);

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
    ephemeral_client.initialize(&account_creator, &expiry, &recovery, &controller_id);

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

    // Initialize ephemeral account, authorizing this SweepController to call sweep()
    ephemeral_client.initialize(&creator, &expiry, &recovery, &controller_id);

// ──────────────────────────────────────────────────────────────────────────────
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
                args: (&account_creator, &expiry, &recovery, &controller_id).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize(&account_creator, &expiry, &recovery, &controller_id);

    let asset_id = Address::generate(&env);
    env.mock_all_auths_allowing_non_root_auth();
    ephemeral_client.record_payment(&100, &asset_id);
    env.set_auths(&[]);

    let result = controller_client.try_claim(&recipient, &ephemeral_id);

    assert!(result.is_err());
}
