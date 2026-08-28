#![cfg(test)]

extern crate std;

use super::*;
use bridgelet_shared::AccountInitRequest;
use ephemeral_account::EphemeralAccountContract;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, BytesN, Env, IntoVal, InvokeError,
};

/// `Env::default()` starts with an all-zero ledger network id, but
/// `ephemeral_account::initialize` (called by `batch_initialize`) enforces
/// `bridgelet_shared::passphrase::require_network`, so any test that
/// exercises `batch_initialize` needs the standalone network id set first.
/// Matches the `test_env()` helper already used in
/// `sweep_controller/tests/integration.rs`.
fn test_env() -> Env {
    let env = Env::default();
    env.ledger()
        .set_network_id(bridgelet_shared::passphrase::standalone_network_id(&env));
    env
}

// Include the compiled ephemeral account WASM so the factory can deploy it
// during tests without depending on `stellar contract build` having run.
// Path is relative to `contracts/account_factory/src/test.rs`.
const EPHEMERAL_ACCOUNT_WASM: &[u8] =
    include_bytes!("../../../target/wasm32v1-none/release/ephemeral_account.wasm");

/// Upload the ephemeral account WASM into the test env, returning both the
/// WASM hash (which the factory will forward to `deploy_v2`) and the
/// template contract id (used for direct SDK calls when convenient).
fn register_template(env: &Env) -> (BytesN<32>, Address) {
    let wasm_hash = env.deployer().upload_contract_wasm(EPHEMERAL_ACCOUNT_WASM);
    let template_id = env.register(EphemeralAccountContract, ());
    (wasm_hash, template_id)
}

fn build_requests(env: &Env, count: u32) -> (u32, Vec<AccountInitRequest>) {
    let expiry = env.ledger().sequence() + 1000;
    let mut reqs = Vec::new(env);
    for i in 0..count {
        reqs.push_back(AccountInitRequest {
            expiry_ledger: expiry + i,
            recovery_address: Address::generate(env),
            authorized_controller: Address::generate(env),
        });
    }
    (expiry, reqs)
}

/// Assert that a slice of addresses contains no duplicates. `Soroban`'s
/// `Address` does not implement std's `Hash`, so we use a Vec + linear scan.
/// The per-batch sizes in these tests are small (≤ 5), so this is O(n²).
fn assert_unique_addresses(addresses: &[Address]) {
    for (i, a) in addresses.iter().enumerate() {
        for (j, b) in addresses.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "addresses at indices {i} and {j} collide");
            }
        }
    }
}

// ── Issue #240: initialize auth + already-initialized guard ──────────────────

#[test]
fn test_initialize_rejects_double_init() {
    let env = test_env();
    env.mock_all_auths();

    let (wasm_hash, _template) = register_template(&env);
    let factory_id = env.register(AccountFactory, ());
    let client = AccountFactoryClient::new(&env, &factory_id);

    let creator = Address::generate(&env);
    client.initialize(&creator, &wasm_hash);

    // Second init — even with the same hash, must be rejected.
    let second = client.try_initialize(&creator, &wasm_hash);
    assert!(matches!(second, Err(Ok(Error::AlreadyInitialized))));
}

#[test]
fn test_initialize_rejects_double_init_with_different_creator() {
    let env = test_env();
    env.mock_all_auths();

    let (wasm_hash, _template) = register_template(&env);
    let factory_id = env.register(AccountFactory, ());
    let client = AccountFactoryClient::new(&env, &factory_id);

    let creator_a = Address::generate(&env);
    let creator_b = Address::generate(&env);

    client.initialize(&creator_a, &wasm_hash);

    // Front-running scenario: a stranger tries to overwrite the hash with
    // their own WASM. Even when every auth is mocked, the guard must win.
    let front_run = client.try_initialize(&creator_b, &wasm_hash);
    assert!(matches!(front_run, Err(Ok(Error::AlreadyInitialized))));
}

#[test]
fn test_initialize_requires_creator_authorization() {
    let env = test_env();
    // Note: no env.mock_all_auths() — real auth path.

    let (wasm_hash, _template) = register_template(&env);
    let factory_id = env.register(AccountFactory, ());
    let client = AccountFactoryClient::new(&env, &factory_id);

    let creator = Address::generate(&env);
    let result = client.try_initialize(&creator, &wasm_hash);

    assert!(matches!(result, Err(Err(InvokeError::Abort))));
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_initialize_panics_with_numeric_code_on_double_init() {
    let env = test_env();
    env.mock_all_auths();

    let (wasm_hash, _template) = register_template(&env);
    let factory_id = env.register(AccountFactory, ());
    let client = AccountFactoryClient::new(&env, &factory_id);

    let creator = Address::generate(&env);
    client.initialize(&creator, &wasm_hash);
    // Non-try call surfaces the contract error directly.
    client.initialize(&creator, &wasm_hash);
}

// ── Issue #241: salt uniqueness across batch_initialize calls ────────────────

#[test]
fn test_batch_initialize_returns_one_success_per_request() {
    let env = test_env();
    env.mock_all_auths();

    let (wasm_hash, _template) = register_template(&env);
    let factory_id = env.register(AccountFactory, ());
    let client = AccountFactoryClient::new(&env, &factory_id);

    let creator = Address::generate(&env);
    client.initialize(&creator, &wasm_hash);

    let (_expiry, requests) = build_requests(&env, 3);
    let results = client.batch_initialize(&creator, &requests);

    assert_eq!(results.len(), requests.len());

    let mut addresses: std::vec::Vec<Address> = std::vec::Vec::new();
    for (i, r) in results.iter().enumerate() {
        assert!(r.success, "request {i} should have succeeded");
        assert!(
            r.error.is_none(),
            "successful request {i} should carry no error"
        );
        addresses.push(r.account_address.clone());

        // Account was initialized with its own per-request authorized_controller
        // (#430) and the factory's fixed admin placeholder; check status to
        // confirm init actually executed rather than leaving an
        // un-initialized placeholder.
        let ephemeral_client =
            ephemeral_account::EphemeralAccountContractClient::new(&env, &r.account_address);
        let status = ephemeral_client.get_status();
        assert_eq!(
            status,
            bridgelet_shared::AccountStatus::Active,
            "deployed account {i} should be in Active state after init"
        );
    }
    assert_unique_addresses(&addresses);
}

#[test]
fn test_batch_initialize_call_nonce_produces_unique_salts_across_calls() {
    let env = test_env();
    env.mock_all_auths();

    let (wasm_hash, _template) = register_template(&env);
    let factory_id = env.register(AccountFactory, ());
    let client = AccountFactoryClient::new(&env, &factory_id);

    let creator = Address::generate(&env);
    client.initialize(&creator, &wasm_hash);

    // First invocation: deploy one account at index 0.
    let (_e1, reqs1) = build_requests(&env, 1);
    let res1 = client.batch_initialize(&creator, &reqs1);
    assert!(res1.get(0).unwrap().success);

    // Second invocation at the same index 0 — in the old code salt[28..32]
    // repeated, colliding with the first invocation. With the per-call nonce
    // these addresses must differ.
    let (_e2, reqs2) = build_requests(&env, 1);
    let res2 = client.batch_initialize(&creator, &reqs2);

    let addr_a = res1.get(0).unwrap().account_address.clone();
    let addr_b = res2.get(0).unwrap().account_address.clone();
    assert_ne!(
        addr_a, addr_b,
        "separate batch_initialize calls at the same index must produce distinct addresses"
    );
}

#[test]
fn test_batch_initialize_keeps_nonce_monotonic_across_more_invocations() {
    let env = test_env();
    env.mock_all_auths();

    let (wasm_hash, _template) = register_template(&env);
    let factory_id = env.register(AccountFactory, ());
    let client = AccountFactoryClient::new(&env, &factory_id);

    let creator = Address::generate(&env);
    client.initialize(&creator, &wasm_hash);

    let (_e1, reqs_a) = build_requests(&env, 1);
    let (_e2, reqs_b) = build_requests(&env, 2);
    let (_e3, reqs_c) = build_requests(&env, 1);

    client.batch_initialize(&creator, &reqs_a);
    client.batch_initialize(&creator, &reqs_b);
    let res_a = client.batch_initialize(&creator, &reqs_a);
    let res_c = client.batch_initialize(&creator, &reqs_c);

    // Each invocation must produce a fresh address for index 0; addresses
    // from the 3rd and 5th invocations must differ.
    assert_ne!(
        res_a.get(0).unwrap().account_address,
        res_c.get(0).unwrap().account_address,
        "repeated invocations should each advance the nonce"
    );

    // Confirm the batch of 2 also produced two unique addresses.
    let mut addresses: std::vec::Vec<Address> = std::vec::Vec::new();
    for r in res_a.iter() {
        addresses.push(r.account_address.clone());
    }
    assert_unique_addresses(&addresses);
}

// ── Issue #425: Error serialization in batch_initialize results ─────────────

/// When try_initialize fails, the error field of AccountInitResult must
/// carry the serialized error code rather than None. (#425)
#[test]
fn test_batch_initialize_serializes_error_on_failure() {
    let env = test_env();
    env.mock_all_auths();

    let (wasm_hash, _template) = register_template(&env);
    let factory_id = env.register(AccountFactory, ());
    let client = AccountFactoryClient::new(&env, &factory_id);

    let creator = Address::generate(&env);
    client.initialize(&creator, &wasm_hash);

    // Build a request with an expiry_ledger in the past so try_initialize
    // returns Err(Ok(Error::InvalidExpiry)). Advance the ledger first --
    // `Env::default()` starts the sequence at 0, so subtracting 1 directly
    // underflows a u32 and panics.
    env.ledger().set_sequence_number(1000);
    let past_expiry = env.ledger().sequence() - 1;
    let mut reqs = Vec::new(&env);
    reqs.push_back(AccountInitRequest {
        expiry_ledger: past_expiry,
        recovery_address: Address::generate(&env),
        authorized_controller: Address::generate(&env),
    });

    let results = client.batch_initialize(&creator, &reqs);
    assert_eq!(results.len(), 1);

    let r = results.get(0).unwrap();
    assert!(!r.success, "request with past expiry should fail");
    assert!(
        r.error.is_some(),
        "error field must be populated for failed init"
    );

    // The serialized error is 4 bytes big-endian u32.  Decode it and
    // verify it matches ephemeral_account::Error::InvalidExpiry (1004).
    let err_bytes = r.error.as_ref().unwrap();
    assert_eq!(err_bytes.len(), 4);
    let code = ((err_bytes.get(0).unwrap() as u32) << 24)
        | ((err_bytes.get(1).unwrap() as u32) << 16)
        | ((err_bytes.get(2).unwrap() as u32) << 8)
        | (err_bytes.get(3).unwrap() as u32);
    assert_eq!(
        code,
        ephemeral_account::Error::InvalidExpiry as u32,
        "serialized error code should match ephemeral_account::Error::InvalidExpiry"
    );
}

/// Successful results must still carry error: None. (#425)
#[test]
fn test_batch_initialize_success_has_no_error() {
    let env = test_env();
    env.mock_all_auths();

    let (wasm_hash, _template) = register_template(&env);
    let factory_id = env.register(AccountFactory, ());
    let client = AccountFactoryClient::new(&env, &factory_id);

    let creator = Address::generate(&env);
    client.initialize(&creator, &wasm_hash);

    let (_expiry, requests) = build_requests(&env, 2);
    let results = client.batch_initialize(&creator, &requests);

    for (i, r) in results.iter().enumerate() {
        assert!(r.success, "request {i} should succeed");
        assert!(r.error.is_none(), "successful request {i} should have no error");
    }
}

/// batch_initialize can be called multiple times without collision. (#423)
#[test]
fn test_batch_initialize_multiple_calls_succeed() {
    let env = test_env();
    env.mock_all_auths();

    let (wasm_hash, _template) = register_template(&env);
    let factory_id = env.register(AccountFactory, ());
    let client = AccountFactoryClient::new(&env, &factory_id);

    let creator = Address::generate(&env);
    client.initialize(&creator, &wasm_hash);

    // Three separate batch calls, each with 2 requests.
    let mut all_addresses: std::vec::Vec<Address> = std::vec::Vec::new();
    for batch in 0..3 {
        let (_expiry, requests) = build_requests(&env, 2);
        let results = client.batch_initialize(&creator, &requests);
        assert_eq!(results.len(), 2, "batch {batch} should return 2 results");
        for (i, r) in results.iter().enumerate() {
            assert!(
                r.success,
                "batch {batch} request {i} should succeed"
            );
            all_addresses.push(r.account_address.clone());
        }
    }
    // All 6 addresses across 3 batches must be unique.
    assert_unique_addresses(&all_addresses);
}

// ── Issue #430: AccountInitRequest carries a per-account authorized_controller ─

/// Two requests in the same batch with different `authorized_controller`
/// values must each wire the matching controller onto their own account, and
/// no other. This is the regression test for the hardcoded-controller bug:
/// on the old code (`authorized_controller` always set to `creator`),
/// `controller_a` would incorrectly be authorized on `account_b` too.
#[test]
fn test_batch_initialize_wires_distinct_authorized_controller_per_request() {
    let env = test_env();
    env.mock_all_auths();

    let (wasm_hash, _template) = register_template(&env);
    let factory_id = env.register(AccountFactory, ());
    let client = AccountFactoryClient::new(&env, &factory_id);

    let creator = Address::generate(&env);
    client.initialize(&creator, &wasm_hash);

    let expiry = env.ledger().sequence() + 1000;
    let controller_a = Address::generate(&env);
    let controller_b = Address::generate(&env);

    let mut reqs = Vec::new(&env);
    reqs.push_back(AccountInitRequest {
        expiry_ledger: expiry,
        recovery_address: Address::generate(&env),
        authorized_controller: controller_a.clone(),
    });
    reqs.push_back(AccountInitRequest {
        expiry_ledger: expiry,
        recovery_address: Address::generate(&env),
        authorized_controller: controller_b.clone(),
    });

    let results = client.batch_initialize(&creator, &reqs);
    let account_a = results.get(0).unwrap().account_address;
    let account_b = results.get(1).unwrap().account_address;

    let client_a = ephemeral_account::EphemeralAccountContractClient::new(&env, &account_a);
    let client_b = ephemeral_account::EphemeralAccountContractClient::new(&env, &account_b);

    // Turn off blanket auth mocking -- from here on only the exact mocked
    // auth below is available, matching how a real deployment behaves.
    env.set_auths(&[]);

    let asset = Address::generate(&env);

    // controller_b (account_b's controller) must NOT be able to record a
    // payment on account_a. On the hardcoded-controller bug this would have
    // succeeded, since both accounts would share the same (creator) controller.
    let rejected = client_a
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &controller_b,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &account_a,
                fn_name: "record_payment",
                args: (500i128, asset.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .try_record_payment(&500, &asset);
    assert!(
        matches!(rejected, Err(Err(InvokeError::Abort))),
        "account_a must not accept controller_b's authorization"
    );

    // account_a's own controller, controller_a, must be able to record it.
    client_a
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &controller_a,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &account_a,
                fn_name: "record_payment",
                args: (500i128, asset.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .record_payment(&500, &asset);

    assert_eq!(
        client_a.get_status(),
        bridgelet_shared::AccountStatus::PaymentReceived
    );

    // Symmetrically, controller_a (account_a's controller) must not be able
    // to record a payment on account_b.
    let rejected_reverse = client_b
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &controller_a,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &account_b,
                fn_name: "record_payment",
                args: (500i128, asset.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .try_record_payment(&500, &asset);
    assert!(
        matches!(rejected_reverse, Err(Err(InvokeError::Abort))),
        "account_b must not accept controller_a's authorization"
    );

    // account_b's own controller, controller_b, must be able to record it.
    client_b
        .mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &controller_b,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &account_b,
                fn_name: "record_payment",
                args: (500i128, asset.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .record_payment(&500, &asset);

    assert_eq!(
        client_b.get_status(),
        bridgelet_shared::AccountStatus::PaymentReceived
    );
}
