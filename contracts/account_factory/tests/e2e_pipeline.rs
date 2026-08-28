#![cfg(test)]

extern crate std;

use account_factory::{AccountFactory, AccountFactoryClient};
use bridgelet_shared::AccountInitRequest;
use ephemeral_account::{AccountStatus, EphemeralAccountContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    vec, Address, BytesN, Env, IntoVal,
};
use sweep_controller::{SweepController, SweepControllerClient};

mod ephemeral_account_wasm {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/ephemeral_account.wasm"
    );
}

/// Issue #250 / #431: end-to-end pipeline test exercising all three live
/// contracts together via the real production entry points --
/// AccountFactory::batch_initialize (not a hand-constructed
/// EphemeralAccountContract via env.register), followed by funding, followed
/// by SweepController::claim.
///
/// This used to fail (tracked as the hardcoded-controller bug, issue #430):
/// AccountFactory::batch_initialize hardcoded `creator` as
/// `authorized_controller` when initializing the ephemeral account, instead
/// of passing the real SweepController contract address, so the deployed
/// account never trusted SweepController to invoke `sweep_claim`. Now that
/// `AccountInitRequest` carries a per-request `authorized_controller` (#430),
/// this test passes the real `controller_id` and asserts on the post-claim
/// state instead of expecting a panic.
#[test]
fn test_full_pipeline_create_via_factory_fund_and_sweep() {
    let env = Env::default();
    // `ephemeral_account::initialize` and `SweepController::initialize` both
    // enforce `bridgelet_shared::passphrase::require_network`, which fails
    // against the default all-zero network id, so the standalone network id
    // must be set before either is called (matches the `test_env()` helper
    // in `sweep_controller/tests/integration.rs`).
    env.ledger()
        .set_network_id(bridgelet_shared::passphrase::standalone_network_id(&env));
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
            // Real SweepController contract address (#430) -- this is the
            // account's authorized_controller from here on.
            authorized_controller: controller_id.clone(),
        },
    ];

    let results = factory_client.batch_initialize(&factory_creator, &requests);

    let result = results.get(0).unwrap();
    if !result.success {
        // Decode the serialized error code (same big-endian u32 encoding
        // used in account_factory::batch_initialize and asserted on in
        // account_factory/src/test.rs's own error-serialization test) so a
        // failure here says *why*, instead of just "false".
        let code: u32 = match result.error.as_ref() {
            Some(err_bytes) if err_bytes.len() == 4 => {
                ((err_bytes.get(0).unwrap() as u32) << 24)
                    | ((err_bytes.get(1).unwrap() as u32) << 16)
                    | ((err_bytes.get(2).unwrap() as u32) << 8)
                    | (err_bytes.get(3).unwrap() as u32)
            }
            _ => 0,
        };
        panic!(
            "factory should successfully deploy and initialize the account \
             (error code: {code}, 0xFFFFFFFF means the outer call itself \
             failed rather than returning a clean contract error)"
        );
    }

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
    // The account's authorized_controller is controller_id (#430), so
    // SweepController's own cross-contract auth on sweep_claim is honored.
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
