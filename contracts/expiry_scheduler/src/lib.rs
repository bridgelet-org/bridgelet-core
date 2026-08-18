#![no_std]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

pub use errors::Error;
pub use storage::ScheduleEntry;

pub trait ExpirySchedulerInterface {
    fn initialize(env: Env, admin: Address) -> Result<(), Error>;
    fn register(env: Env, account: Address, expiry_ledger: u32) -> Result<(), Error>;
    fn due_before(env: Env, ledger: u32, max: u32) -> Vec<Address>;
    fn deregister(env: Env, caller: Address, account: Address) -> Result<(), Error>;
}

#[contract]
pub struct ExpiryScheduler;

#[contractimpl]
impl ExpiryScheduler {
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);
        if storage::get_admin(&env).is_some() {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        storage::set_admin(&env, &admin);
        events::emit_initialized(&env, admin);
        Ok(())
    }

    pub fn register(env: Env, account: Address, expiry_ledger: u32) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);
        if storage::get_admin(&env).is_none() {
            return Err(Error::NotInitialized);
        }
        account.require_auth();

        let mut entries = storage::get_entries(&env);
        let mut existing = false;
        for i in 0..entries.len() {
            let entry = entries.get(i).unwrap();
            if entry.account == account {
                entries.remove(i);
                existing = true;
                break;
            }
        }

        let mut insert_at = entries.len();
        for i in 0..entries.len() {
            if entries.get(i).unwrap().expiry_ledger > expiry_ledger {
                insert_at = i;
                break;
            }
        }

        let new_entry = storage::ScheduleEntry {
            account: account.clone(),
            expiry_ledger,
        };
        entries.insert(insert_at, new_entry);
        storage::set_entries(&env, &entries);

        if existing {
            events::emit_rescheduled(&env, account, expiry_ledger);
        } else {
            events::emit_registered(&env, account, expiry_ledger);
        }
        Ok(())
    }

    pub fn due_before(env: Env, ledger: u32, max: u32) -> Vec<Address> {
        storage::extend_instance_ttl(&env);
        let entries = storage::get_entries(&env);
        let mut result = Vec::new(&env);
        for entry in entries.iter() {
            if result.len() >= max || entry.expiry_ledger > ledger {
                break;
            }
            result.push_back(entry.account);
        }
        result
    }

    pub fn deregister(env: Env, caller: Address, account: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);
        let admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
        if caller != account && caller != admin {
            return Err(Error::Unauthorized);
        }
        caller.require_auth();

        let mut entries = storage::get_entries(&env);
        let mut found = false;
        for i in 0..entries.len() {
            if entries.get(i).unwrap().account == account {
                entries.remove(i);
                found = true;
                break;
            }
        }
        if !found {
            return Err(Error::NotRegistered);
        }
        storage::set_entries(&env, &entries);
        events::emit_deregistered(&env, account);
        Ok(())
    }
}

impl ExpirySchedulerInterface for ExpiryScheduler {
    fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        Self::initialize(env, admin)
    }
    fn register(env: Env, account: Address, expiry_ledger: u32) -> Result<(), Error> {
        Self::register(env, account, expiry_ledger)
    }
    fn due_before(env: Env, ledger: u32, max: u32) -> Vec<Address> {
        Self::due_before(env, ledger, max)
    }
    fn deregister(env: Env, caller: Address, account: Address) -> Result<(), Error> {
        Self::deregister(env, caller, account)
    }
}
