#![no_std]

mod ephemeral_account_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/ephemeral_account.wasm"
    );
}
use ephemeral_account_contract::Client as EphemeralAccountClient;

mod errors;
pub use errors::Error;

#[cfg(test)]
mod test;

use bridgelet_shared::{AccountInitRequest, AccountInitResult};
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, Vec};

#[contract]
pub struct AccountFactory;

#[contractimpl]
impl AccountFactory {
    /// Initialize the factory contract (store the ephemeral account contract wasm hash).
    ///
    /// This entry point is **single-shot**: it requires the creator (deployment
    /// authorizer) to provide its address and prove authorization, and it
    /// rejects any second call once the WASM hash has been written (issue
    /// #240). Without these guards any caller could overwrite the stored WASM
    /// hash with a malicious contract before the legitimate operator.
    ///
    /// # Arguments
    /// * `creator` - Authorizing address; the only caller permitted to set the
    ///   factory's WASM hash. Must produce a valid Soroban auth entry.
    /// * `ephemeral_account_wasm_hash` - Hash of the ephemeral account
    ///   contract WASM that subsequent `batch_initialize` calls will deploy.
    ///
    /// # Errors
    /// * `Error::AlreadyInitialized` - factory has already been initialized.
    pub fn initialize(
        env: Env,
        creator: Address,
        ephemeral_account_wasm_hash: BytesN<32>,
    ) -> Result<(), Error> {
        // State check fires BEFORE require_auth so a double-init attempt from
        // any caller is rejected without paying the cost of an auth entry.
        if env
            .storage()
            .instance()
            .has(&DataKey::EphemeralAccountWasmHash)
        {
            return Err(Error::AlreadyInitialized);
        }

        // Creator must authorize the write. After the guard fires above, this
        // is the only call that ever sets the WASM hash.
        creator.require_auth();

        env.storage().instance().set(
            &DataKey::EphemeralAccountWasmHash,
            &ephemeral_account_wasm_hash,
        );
        env.storage().instance().set(&DataKey::BatchNonce, &0u64);

        Ok(())
    }

    /// Batch initialize multiple ephemeral accounts in a single transaction.
    ///
    /// # Arguments
    /// * `creator` - Address creating all accounts
    /// * `requests` - Vector of [`AccountInitRequest`].
    ///
    /// # Salt uniqueness (issue #241)
    /// `Soroban`'s `deployer().with_current_contract(salt)` derives a contract
    /// address deterministically from `(factory_address, salt, wasm_hash)`. A
    /// salt that depended only on the per-batch index would collide the
    /// second time `batch_initialize` was invoked with a request at index 0,
    /// because the loop would once again produce `salt = [0, ..., index]`.
    /// We therefore mix a monotonic per-factory-call counter
    /// (`DataKey::BatchNonce`) into the high bytes of the salt so that
    /// distinct invocations of `batch_initialize` always produce disjoint
    /// address ranges, even at the same index.
    ///
    /// # Returns
    /// Vector of [`AccountInitResult`] preserving the input order.
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
            .expect("factory not initialized; call initialize() first");

        // Bump the per-factory-call nonce exactly once per invocation.
        // The combined `nonce || index` salt ensures no two deployments from
        // separate calls ever produce the same address, while still being
        // deterministic within a single call.
        let prev_nonce: u64 = env
            .storage()
            .instance()
            .get(&DataKey::BatchNonce)
            .unwrap_or(0);
        // u64 + 1 cannot overflow for any realistic call count. The workspace
        // enables `overflow-checks = true` in release, so any overflow would
        // surface as a panic rather than a silent wraparound to a colliding
        // salt.
        let nonce = prev_nonce + 1u64;
        env.storage().instance().set(&DataKey::BatchNonce, &nonce);

        let mut results = Vec::new(&env);

        for (index, request) in requests.iter().enumerate() {
            // Salt layout (32 bytes, big-endian):
            //   [0..8]  nonce   — monotonically increases each call to
            //                     batch_initialize
            //   [8..28] zeros  — reserved (kept zero to leave room for future
            //                    fields such as a creator-tag)
            //   [28..32] index — per-request position inside the call
            let mut salt_bytes = [0u8; 32];
            salt_bytes[0..8].copy_from_slice(&nonce.to_be_bytes());
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
    /// Monotonically increasing counter incremented once per call to
    /// `batch_initialize`. Mixed into the deployment salt to keep addresses
    /// disjoint across separate invocations (issue #241).
    BatchNonce,
}
