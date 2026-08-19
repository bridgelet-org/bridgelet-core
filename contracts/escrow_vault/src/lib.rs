#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, token, Address, Env};

pub use errors::Error;
pub use storage::EscrowEntry;

pub trait EscrowVaultInterface {
    fn open(
        env: Env,
        depositor: Address,
        beneficiary: Address,
        asset: Address,
        amount: i128,
        release_ledger: u32,
    ) -> Result<u64, Error>;

    fn release(env: Env, id: u64) -> Result<(), Error>;

    fn cancel(env: Env, depositor: Address, id: u64) -> Result<(), Error>;

    fn get(env: Env, id: u64) -> Option<EscrowEntry>;
}

#[contract]
pub struct EscrowVault;

#[contractimpl]
impl EscrowVaultInterface for EscrowVault {
    fn open(
        env: Env,
        depositor: Address,
        beneficiary: Address,
        asset: Address,
        amount: i128,
        release_ledger: u32,
    ) -> Result<u64, Error> {
        storage::extend_instance_ttl(&env);

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        depositor.require_auth();

        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&depositor, &env.current_contract_address(), &amount);

        let id = storage::increment_next_id(&env);

        let entry = EscrowEntry {
            depositor: depositor.clone(),
            beneficiary: beneficiary.clone(),
            asset: asset.clone(),
            amount,
            release_ledger,
        };

        storage::set_escrow(&env, id, &entry);
        events::emit_opened(
            &env,
            id,
            depositor,
            beneficiary,
            asset,
            amount,
            release_ledger,
        );

        Ok(id)
    }

    fn release(env: Env, id: u64) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let entry = storage::get_escrow(&env, id).ok_or(Error::NotFound)?;

        if env.ledger().sequence() < entry.release_ledger {
            return Err(Error::TooEarlyToRelease);
        }

        let token_client = token::Client::new(&env, &entry.asset);
        token_client.transfer(
            &env.current_contract_address(),
            &entry.beneficiary,
            &entry.amount,
        );

        storage::remove_escrow(&env, id);
        events::emit_released(&env, id);

        Ok(())
    }

    fn cancel(env: Env, depositor: Address, id: u64) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        let entry = storage::get_escrow(&env, id).ok_or(Error::NotFound)?;

        if entry.depositor != depositor {
            return Err(Error::Unauthorized);
        }

        depositor.require_auth();

        let token_client = token::Client::new(&env, &entry.asset);
        token_client.transfer(
            &env.current_contract_address(),
            &entry.depositor,
            &entry.amount,
        );

        storage::remove_escrow(&env, id);
        events::emit_canceled(&env, id);

        Ok(())
    }

    fn get(env: Env, id: u64) -> Option<EscrowEntry> {
        storage::get_escrow(&env, id)
    }
}
