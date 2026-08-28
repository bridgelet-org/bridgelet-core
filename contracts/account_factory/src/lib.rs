#![no_std]

use bridgelet_shared::{AccountInitRequest, AccountInitResult};
use ephemeral_account::EphemeralAccountContractClient as EphemeralAccountClient;
use soroban_sdk::{contract, contractimpl, contracttype, Address, Bytes, BytesN, Env, Vec};

pub mod errors;
pub use crate::errors::Error;

// `src/test.rs` was never wired into the crate module tree, so every unit
// test in it (double-init guards, salt-uniqueness checks, error
// serialization, and the new #430 regression test) silently compiled out
// and never ran under `cargo test -p account_factory`. Declaring the
// module here matches the pattern already used by `ephemeral_account` and
// `sweep_controller` (`mod test;` + `#![cfg(test)]` inside test.rs itself).
mod test;

/// Minimum remaining TTL (in ledgers) before the instance storage is
/// proactively extended.  Matches the threshold used by the other
/// contracts in this workspace.
const INSTANCE_TTL_THRESHOLD: u32 = 100;

/// Target TTL (in ledgers) after extension.  ~30 days at 5 s/ledger.
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;

/// Extend instance storage TTL so the factory's stored WASM hash and
/// batch nonce remain accessible across long idle periods. (#427)
fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

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
        // Extend TTL on every call so the factory's instance storage
        // (WASM hash, batch nonce) survives long idle periods. (#427)
        extend_instance_ttl(&env);

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
    /// * `requests` - Vector of [`AccountInitRequest`], each carrying its own
    ///   `authorized_controller` (#430) so different accounts in the same
    ///   batch can be wired to different controllers (e.g. distinct
    ///   `SweepController` instances) instead of always trusting `creator`.
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
        // Extend TTL on every call so the factory's instance storage
        // (WASM hash, batch nonce) survives long idle periods. (#427)
        extend_instance_ttl(&env);

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
                // Per-account authorized_controller (#430). Previously this was
                // hardcoded to `creator`, so every factory-created account
                // trusted `creator` and never a real SweepController instance.
                &request.authorized_controller,
                // Placeholder authorized_signer — batch_initialize does not
                // verify off-chain signatures; the creator (admin) manages
                // signer keys via the single-account interface.
                &BytesN::from_array(&env, &[0u8; 32]),
                // Admin is a deterministic placeholder address so `batch_initialize`
                // doesn't depend on the `testutils` feature being enabled. The
                // creator (passed as deployer auth above) is the actual admin in
                // production deployments; this value is here only because the
                // 6-arg `initialize` shape requires an admin slot on every call.
                //
                // NOTE: this must be a validly-checksummed strkey (all-zero
                // ED25519 public key payload) or `Address::from_str` panics with
                // "couldn't process the string as strkey", failing every
                // `batch_initialize` call. The literal here previously had the
                // wrong checksum suffix and had never been exercised, since
                // `src/test.rs` (see `lib.rs`'s `mod test;`) was not wired into
                // this crate until now.
                &Address::from_str(
                    &env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
                // No reserve_contract — factory-created accounts use the
                // compile-time default base reserve (Issue #405).
                &None::<Address>,
            ) {
                Ok(_) => AccountInitResult {
                    account_address: account_address.clone(),
                    success: true,
                    error: None,
                },
                Err(err) => {
                    // Serialize the error code so callers can distinguish
                    // failure reasons instead of seeing a bare None. (#425)
                    //
                    // try_initialize returns Result<Result<(), ContractError>,
                    // InvokeError>.  Contract errors carry a u32 discriminant;
                    // host/invocation errors are opaque and encoded as 0xFFFF_FFFF.
                    let error_code: u32 = match err {
                        Ok(contract_err) => contract_err as u32,
                        Err(_) => 0xFFFF_FFFF,
                    };
                    let mut error_bytes = Bytes::new(&env);
                    error_bytes.push_back(((error_code >> 24) & 0xFF) as u8);
                    error_bytes.push_back(((error_code >> 16) & 0xFF) as u8);
                    error_bytes.push_back(((error_code >> 8) & 0xFF) as u8);
                    error_bytes.push_back((error_code & 0xFF) as u8);
                    AccountInitResult {
                        account_address: account_address.clone(),
                        success: false,
                        error: Some(error_bytes),
                    }
                }
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
