#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, xdr::ToXdr, Address, Bytes, BytesN, Env};

pub use errors::Error;
pub use events::AccountDeployed;
pub use storage::DataKey;

#[contract]
pub struct AccountFactoryContract;

#[contractimpl]
impl AccountFactoryContract {
    /// Initialize the factory with admin and ephemeral account wasm hash.
    pub fn initialize(
        env: Env,
        admin: Address,
        ephemeral_wasm_hash: BytesN<32>,
    ) -> Result<(), Error> {
        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();

        storage::set_admin(&env, &admin);
        storage::set_ephemeral_wasm_hash(&env, &ephemeral_wasm_hash);

        Ok(())
    }

    /// Deploy and initialize a new ephemeral account contract instance.
    pub fn create_account(
        env: Env,
        creator: Address,
        expiry_ledger: u32,
        recovery_address: Address,
        native_transfer_address: Address,
        claim_verifier_address: Address,
    ) -> Result<Address, Error> {
        let wasm_hash = storage::get_ephemeral_wasm_hash(&env).ok_or(Error::NotInitialized)?;

        creator.require_auth();

        let salt = Self::account_salt(&env, &creator, expiry_ledger);
        let deployed_address = env
            .deployer()
            .with_current_contract(salt)
            .deploy_v2(wasm_hash, ());

        let account_client =
            ephemeral_account::EphemeralAccountContractClient::new(&env, &deployed_address);
        account_client.initialize(
            &creator,
            &expiry_ledger,
            &recovery_address,
            &native_transfer_address,
            &claim_verifier_address,
        );

        storage::add_deployed_account(&env, &creator, &deployed_address);
        events::emit_account_deployed(&env, deployed_address.clone(), creator, expiry_ledger);

        Ok(deployed_address)
    }

    /// Get all accounts deployed by a specific creator.
    pub fn get_accounts_by_creator(env: Env, creator: Address) -> soroban_sdk::Vec<Address> {
        storage::get_accounts_by_creator(&env, &creator)
    }

    /// Get all deployed accounts.
    pub fn get_all_accounts(env: Env) -> soroban_sdk::Vec<Address> {
        storage::get_all_accounts(&env)
    }

    fn account_salt(env: &Env, creator: &Address, expiry_ledger: u32) -> BytesN<32> {
        let mut salt_preimage = Bytes::new(env);
        salt_preimage.append(&creator.to_xdr(env));
        salt_preimage.append(&expiry_ledger.to_xdr(env));
        salt_preimage.append(&env.ledger().sequence().to_xdr(env));

        env.crypto().sha256(&salt_preimage).into()
    }
}
