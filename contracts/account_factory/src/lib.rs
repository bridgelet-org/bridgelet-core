#![no_std]

use bridgelet_shared::{AccountInitRequest, AccountInitResult, PaginatedAccountInitResultResponse, PaginationParams, NO_CURSOR};

mod ephemeral_account_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/ephemeral_account.wasm"
    );
}
use ephemeral_account_contract::Client as EphemeralAccountClient;

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

    /// Batch initialize multiple ephemeral accounts in a single transaction.
    /// Returns all results at once (unpaginated).
    ///
    /// # Arguments
    /// * `creator` - Address creating all accounts
    /// * `requests` - Vector of AccountInitRequest
    ///
    /// # Returns
    /// Vector of AccountInitResult
    ///
    /// # Warning
    /// This function returns all results in a single response. For large batches,
    /// prefer `batch_initialize_paginated` to avoid resource limit failures.
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
            let mut salt_bytes = [0u8; 32];
            salt_bytes[28..32].copy_from_slice(&(index as u32).to_be_bytes());
            let salt = BytesN::from_array(&env, &salt_bytes);
            let account_address = env
                .deployer()
                .with_current_contract(salt)
                .deploy_v2(wasm_hash.clone(), ());

            let client = EphemeralAccountClient::new(&env, &account_address);

            let result = match client.try_initialize(
                &creator,
                &request.expiry_ledger,
                &request.recovery_address,
                &creator,
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
                    error: None,
                },
            };

            results.push_back(result);
        }

        results
    }

    /// Batch initialize multiple ephemeral accounts with cursor-based pagination.
    /// Processes a subset of requests and returns a page of results with a cursor
    /// for the next page.
    ///
    /// # Arguments
    /// * `creator` - Address creating all accounts
    /// * `requests` - Vector of AccountInitRequest
    /// * `params` - Pagination parameters (limit, cursor_index)
    ///
    /// # Returns
    /// PaginatedAccountInitResultResponse containing a page of AccountInitResult items and
    /// a next_cursor_index (NO_CURSOR if no more pages).
    ///
    /// # Example
    /// ```ignore
    /// // First page (limit 10)
    /// let page1 = factory.batch_initialize_paginated(
    ///     creator.clone(),
    ///     requests.clone(),
    ///     PaginationParams { limit: 10, cursor_index: NO_CURSOR }
    /// );
    ///
    /// // Subsequent pages
    /// if page1.next_cursor_index != NO_CURSOR {
    ///     let page2 = factory.batch_initialize_paginated(
    ///         creator.clone(),
    ///         requests.clone(),
    ///         PaginationParams { limit: 10, cursor_index: page1.next_cursor_index }
    ///     );
    /// }
    /// ```
    pub fn batch_initialize_paginated(
        env: Env,
        creator: Address,
        requests: Vec<AccountInitRequest>,
        params: PaginationParams,
    ) -> PaginatedAccountInitResultResponse {
        creator.require_auth();

        let wasm_hash = env
            .storage()
            .instance()
            .get::<_, BytesN<32>>(&DataKey::EphemeralAccountWasmHash)
            .unwrap();

        let total_count = requests.len() as u32;
        let limit = params.limit.min(1000).max(1) as u32;
        let start_index = if params.cursor_index == NO_CURSOR { 0 } else { params.cursor_index };
        let end_index = (start_index + limit).min(total_count);

        let mut results = Vec::new(&env);

        for index in start_index..end_index {
            let request = requests.get(index).unwrap();
            let mut salt_bytes = [0u8; 32];
            salt_bytes[28..32].copy_from_slice(&(index).to_be_bytes());
            let salt = BytesN::from_array(&env, &salt_bytes);
            let account_address = env
                .deployer()
                .with_current_contract(salt)
                .deploy_v2(wasm_hash.clone(), ());

            let client = EphemeralAccountClient::new(&env, &account_address);

            let result = match client.try_initialize(
                &creator,
                &request.expiry_ledger,
                &request.recovery_address,
                &creator,
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
                    error: None,
                },
            };

            results.push_back(result);
        }

        let next_cursor_index = if end_index < total_count {
            end_index
        } else {
            NO_CURSOR
        };

        PaginatedAccountInitResultResponse {
            items: results,
            next_cursor_index,
            total_count,
        }
    }

    /// Get the total number of accounts that would be created for a given
    /// set of requests (without actually creating them).
    ///
    /// # Arguments
    /// * `_env` - Soroban environment (unused, for interface consistency)
    /// * `requests` - Vector of AccountInitRequest
    ///
    /// # Returns
    /// Total count of accounts to be created.
    pub fn batch_initialize_count(_env: Env, requests: Vec<AccountInitRequest>) -> u32 {
        requests.len() as u32
    }
}

#[contracttype]
enum DataKey {
    EphemeralAccountWasmHash,
}
