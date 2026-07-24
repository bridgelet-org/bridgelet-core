extern crate std;

use super::*;
use bridgelet_shared::AccountInitRequest;
use ephemeral_account::EphemeralAccountContract;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, InvokeError};

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
    let env = Env::default();
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
    let env = Env::default();
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
    let env = Env::default();
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
    let env = Env::default();
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
    let env = Env::default();
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

        // Account was initialized with the creator as authorized_controller
        // and admin per the factory's wiring; check status to confirm init
        // actually executed rather than leaving an un-initialized placeholder.
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
    let env = Env::default();
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
    let env = Env::default();
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
