#![no_std]

use bridgelet_shared::{AccountInitRequest, AccountInitResult};
use ephemeral_account::EphemeralAccountContractClient as EphemeralAccountClient;
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, Vec};

#[contract]
pub struct AccountFactory;

#[contractimpl]
impl AccountFactory {
    /// Initialize the factory contract (store the ephemeral account contract wasm hash)
    ///
    /// # Arguments
    /// * `ephemeral_account_wasm_hash` - Hash of the ephemeral account contract wasm
    pub fn initialize(env: Env, ephemeral_account_wasm_hash: BytesN<32>) {
        env.storage().instance().set(
            &DataKey::EphemeralAccountWasmHash,
            &ephemeral_account_wasm_hash,
        );
    }

    /// Batch initialize multiple ephemeral accounts in a single transaction
    ///
    /// # Arguments
    /// * `creator` - Address creating all accounts
    /// * `requests` - Vector of AccountInitRequest
    ///
    /// # Returns
    /// Vector of AccountInitResult
    pub fn batch_initialize(
        env: Env,
        creator: Address,
        requests: Vec<AccountInitRequest>,
    ) -> Vec<AccountInitResult> {
        creator.require_auth();

        let wasm_hash = env
            .storage()
            .instance()
            .get::<_, BytesN<32>>(&DataKey::EphemeralAccountWasmHash)
            .unwrap();

        let mut results = Vec::new(&env);

        for (index, request) in requests.iter().enumerate() {
            // Deploy a new ephemeral account contract with unique salt
            let mut salt_bytes = [0u8; 32];
            salt_bytes[28..32].copy_from_slice(&(index as u32).to_be_bytes());
            let salt = BytesN::from_array(&env, &salt_bytes);
            let account_address = env
                .deployer()
                .with_current_contract(salt)
                .deploy_v2(wasm_hash.clone(), ());

            // Initialize it
            let client = EphemeralAccountClient::new(&env, &account_address);

            let result = match client.try_initialize(
                &creator,
                &request.expiry_ledger,
                &request.recovery_address,
                &creator,
            ) {
                Ok(_) => AccountInitResult {
                    account_address: account_address.clone(),
                    success: true,
                    error: None,
                },
                Err(_) => AccountInitResult {
                    account_address: account_address.clone(),
                    success: false,
                    error: None, // In a real implementation, we'd serialize errors
                },
            };

            results.push_back(result);
        }

        results
    }
}

#[contracttype]
enum DataKey {
    EphemeralAccountWasmHash,
}
