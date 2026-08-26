#![cfg(batch_initialize)]

extern crate std;

use account_factory::{AccountFactory, AccountFactoryClient};
use bridgelet_shared::AccountInitRequest;
use ephemeral_account::{AccountStatus, EphemeralAccountContractClient};
use soroban_sdk::{testutils::Address as _, vec, Address, BytesN, Env, IntoVal};
use sweep_controller::{SweepController, SweepControllerClient};

mod ephemeral_account_wasm {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/ephemeral_account.wasm"
    );
}

/// Issue #250: end-to-end pipeline test exercising all three live contracts
/// together via the real production entry points -- AccountFactory::batch_initialize
/// (not a hand-constructed EphemeralAccountContract via env.register), followed by
/// funding, followed by SweepController::claim.
///
/// KNOWN FAILING TODAY (tracked as the hardcoded-controller bug, see issue
/// referenced from #250): AccountFactory::batch_initialize hardcodes `creator`
/// as both `authorized_controller` and `admin` when initializing the ephemeral
/// account, instead of passing the real SweepController contract address. That
/// means the deployed account never trusts SweepController to invoke
/// `sweep_claim`, so SweepController::claim panics with an authorization
/// failure once the flow runs through the real factory instead of being
/// hand-wired, as every other test in this codebase does.
///
/// IMPORTANT: once the controller-address bug is fixed, this test will stop
/// panicking. At that point, remove the `#[should_panic]` attribute below and
/// replace it with real assertions on the post-claim state (status == Swept,
/// swept_to == recipient) -- do not leave this marked should_panic once the
/// underlying bug is fixed, or this test stops providing any coverage.
#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_full_pipeline_create_via_factory_fund_and_sweep() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    // -- Deploy AccountFactory and upload the real ephemeral_account wasm --
    let factory_id = env.register(AccountFactory, ());
    let factory_client = AccountFactoryClient::new(&env, &factory_id);

    let factory_creator = Address::generate(&env);
    let wasm_hash = env
        .deployer()
        .upload_contract_wasm(ephemeral_account_wasm::WASM);
    factory_client.initialize(&factory_creator, &wasm_hash);

    // -- Deploy and initialize SweepController (locked to `recipient`) --
    let controller_id = env.register(SweepController, ());
    let controller_client = SweepControllerClient::new(&env, &controller_id);

    let controller_creator = Address::generate(&env);
    let authorized_signer = BytesN::from_array(&env, &[7u8; 32]);
    let recipient = Address::generate(&env);

    controller_client.initialize(
        &controller_creator,
        &authorized_signer,
        &Some(recipient.clone()),
    );

    // -- Create an ephemeral account via the REAL factory path, not by hand --
    let recovery = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1_000;

    let requests = vec![
        &env,
        AccountInitRequest {
            expiry_ledger: expiry,
            recovery_address: recovery.clone(),
        },
    ];

    let results = factory_client.batch_initialize(&factory_creator, &requests);

    let result = results.get(0).unwrap();
    assert!(
        result.success,
        "factory should successfully deploy and initialize the account"
    );

    let account_address = result.account_address;
    let ephemeral_client = EphemeralAccountContractClient::new(&env, &account_address);

    // -- Fund the account (record_payment requires no auth) --
    let asset = Address::generate(&env);
    ephemeral_client.record_payment(&500, &asset);

    assert_eq!(
        ephemeral_client.get_status(),
        AccountStatus::PaymentReceived
    );

    // Reset blanket auth mocking -- from here on, only the recipient's claim
    // auth is provided, exactly as a real deployment would look.
    env.set_auths(&[]);

    // -- Sweep via SweepController::claim, with ONLY the recipient's auth mocked --
    // If the account's authorized_controller doesn't match the real SweepController
    // address (the hardcoded-controller bug), this call panics with an
    // authorization failure, since nothing authorized `factory_creator` (the
    // value wrongly stored as authorized_controller) to approve this claim.
    controller_client
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &recipient,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &controller_id,
                fn_name: "claim",
                args: (&recipient, &account_address).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .claim(&recipient, &account_address);

    assert_eq!(ephemeral_client.get_status(), AccountStatus::Swept);
    assert_eq!(ephemeral_client.get_info().swept_to, Some(recipient));
}
