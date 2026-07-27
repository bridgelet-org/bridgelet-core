#![cfg(test)]
use soroban_sdk::{testutils::Address as _, Address, Env};
use crate::{BridgeletCore, BridgeletCoreClient}; 

mod mock_v2 {
    soroban_sdk::contractimport!(
        file = "./test_fixtures/mock_v2/target/wasm32v1-none/release/mock_v2.wasm"
    );
}

#[test]
fn test_contract_upgrade_preserves_state_and_adds_feature() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(BridgeletCore, ());
    let client = BridgeletCoreClient::new(&env, &contract_id);

    client.initialize(&admin);
    client.set_base_reserve(&5000);
    
    let reserve_before = client.get_base_reserve();
    assert_eq!(reserve_before, 5000, "V1 state should be set correctly");

    let v2_wasm_hash = env.deployer().upload_contract_wasm(mock_v2::WASM);

    client.upgrade(&v2_wasm_hash);

    let v2_client = mock_v2::Client::new(&env, &contract_id);
    
    let reserve_after = v2_client.get_base_reserve();
    assert_eq!(reserve_after, 5000, "V2 must retain V1 state");

    let new_val = v2_client.v2_exclusive_feature();
    assert_eq!(new_val, 100, "V2 exclusive feature must be accessible");
}