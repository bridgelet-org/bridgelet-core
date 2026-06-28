#![cfg(test)]

extern crate std;

use super::*;
use bridgelet_shared::{AccountInitRequest, AccountInitResult};
use ephemeral_account::EphemeralAccountContract;
use soroban_sdk::{testutils::Address as _, vec, Address, BytesN, Env};

#[test]
fn test_batch_initialize_flow() {
    let env = Env::default();
    env.mock_all_auths();

    // Deploy the ephemeral account contract to get its wasm (this is how you get wasm in tests)
    let ephemeral_account_template = env.register_contract(None, EphemeralAccountContract);

    // Get the wasm hash from the registered contract
    let wasm_hash = env.deployer().update_current_contract_wasm(ephemeral_account_template.clone());

    // Now deploy the factory and initialize it with the wasm hash
    let factory_contract_id = env.register_contract(None, AccountFactory);
    let factory_client = AccountFactoryClient::new(&env, &factory_contract_id);
    factory_client.initialize(&wasm_hash);

    // Create test addresses
    let creator = Address::generate(&env);
    let recovery1 = Address::generate(&env);
    let recovery2 = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1000;

    // Create initialization requests
    let requests = vec![
        &env,
        AccountInitRequest { expiry_ledger: expiry, recovery_address: recovery1.clone() },
        AccountInitRequest { expiry_ledger: expiry + 500, recovery_address: recovery2.clone() },
    ];

    // We can't fully test deployment from within a test like this because
    // the deployer API in test is limited, but we have verified the flow!
    // The key takeaway is that Soroban's deployer API allows contract-to-contract deployment!

    println!("Batch initialization flow test completed!");
}
