#![no_std]

use crate::storage::Sponsorship;
use soroban_sdk::{contract, contractimpl, Address, Env};

mod errors;
mod events;
mod storage;

pub use errors::Error;

pub trait FeeSponsorInterface {
    fn deposit(env: Env, sponsor: Address, amount: i128) -> Result<(), Error>;
    fn sponsor_account(env: Env, sponsor: Address, account: Address, cap: i128) -> Result<(), Error>;
    fn draw(env: Env, account: Address, amount: i128) -> Result<(), Error>;
    fn remaining(env: Env, account: Address) -> i128;
}

#[contract]
pub struct FeeSponsorContract;

#[contractimpl]
impl FeeSponsorInterface for FeeSponsorContract {
    fn deposit(env: Env, sponsor: Address, amount: i128) -> Result<(), Error> {
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        storage::extend_instance_ttl(&env);
        storage::add_sponsor_balance(&env, &sponsor, amount);
        events::emit_deposit(&env, sponsor, amount);

        Ok(())
    }

    fn sponsor_account(env: Env, sponsor: Address, account: Address, cap: i128) -> Result<(), Error> {
        if cap <= 0 {
            return Err(Error::InvalidAmount);
        }

        storage::extend_instance_ttl(&env);

        // Check if account already has a sponsorship
        if storage::has_sponsorship(&env, &account) {
            return Err(Error::AlreadySponsored);
        }

        // Verify sponsor has sufficient balance
        let sponsor_balance = storage::get_sponsor_balance(&env, &sponsor);
        if sponsor_balance < cap {
            return Err(Error::InsufficientDeposit);
        }

        // Create sponsorship
        let sponsorship = Sponsorship {
            sponsor: sponsor.clone(),
            cap,
            drawn: 0,
        };

        storage::set_sponsorship(&env, &account, &sponsorship);
        events::emit_sponsorship_created(&env, sponsor, account, cap);

        Ok(())
    }

    fn draw(env: Env, account: Address, amount: i128) -> Result<(), Error> {
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        storage::extend_instance_ttl(&env);

        // Get sponsorship
        let mut sponsorship = storage::get_sponsorship(&env, &account)
            .ok_or(Error::AccountNotSponsored)?;

        // Check if draw exceeds remaining cap
        let remaining_cap = sponsorship.cap - sponsorship.drawn;
        if amount > remaining_cap {
            return Err(Error::ExceedsCap);
        }

        // Check if sponsor has sufficient balance
        let sponsor_balance = storage::get_sponsor_balance(&env, &sponsorship.sponsor);
        if amount > sponsor_balance {
            return Err(Error::InsufficientRemaining);
        }

        // Update sponsorship
        sponsorship.drawn += amount;
        storage::set_sponsorship(&env, &account, &sponsorship);

        // Deduct from sponsor balance
        storage::subtract_sponsor_balance(&env, &sponsorship.sponsor, amount);

        let new_remaining = sponsorship.cap - sponsorship.drawn;
        events::emit_draw(&env, account, amount, new_remaining);

        Ok(())
    }

    fn remaining(env: Env, account: Address) -> i128 {
        storage::extend_instance_ttl(&env);

        match storage::get_sponsorship(&env, &account) {
            Some(sponsorship) => sponsorship.cap - sponsorship.drawn,
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env};

    #[test]
    fn test_deposit_and_sponsor_account() {
        let env = Env::default();
        let contract_id = env.register_contract(None, FeeSponsorContract);
        let client = FeeSponsorContractClient::new(&env, &contract_id);

        let sponsor = Address::generate(&env);
        let account = Address::generate(&env);

        // Deposit funds
        client.deposit(&sponsor, &1000);
        assert_eq!(client.remaining(&account), 0);

        // Sponsor account
        client.sponsor_account(&sponsor, &account, &500);
        assert_eq!(client.remaining(&account), 500);
    }

    #[test]
    fn test_draw_within_cap() {
        let env = Env::default();
        let contract_id = env.register_contract(None, FeeSponsorContract);
        let client = FeeSponsorContractClient::new(&env, &contract_id);

        let sponsor = Address::generate(&env);
        let account = Address::generate(&env);

        client.deposit(&sponsor, &1000);
        client.sponsor_account(&sponsor, &account, &500);

        // Draw within cap
        client.draw(&account, &200);
        assert_eq!(client.remaining(&account), 300);

        // Another draw
        client.draw(&account, &100);
        assert_eq!(client.remaining(&account), 200);
    }

    #[test]
    fn test_draw_exceeds_cap() {
        let env = Env::default();
        let contract_id = env.register_contract(None, FeeSponsorContract);
        let client = FeeSponsorContractClient::new(&env, &contract_id);

        let sponsor = Address::generate(&env);
        let account = Address::generate(&env);

        client.deposit(&sponsor, &1000);
        client.sponsor_account(&sponsor, &account, &500);

        // Draw within cap
        client.draw(&account, &300);
        assert_eq!(client.remaining(&account), 200);

        // Try to draw more than remaining cap
        let result = client.try_draw(&account, &300);
        assert_eq!(result, Err(Ok(Error::ExceedsCap)));
        assert_eq!(client.remaining(&account), 200);
    }

    #[test]
    fn test_draw_limited_by_sponsor_balance() {
        let env = Env::default();
        let contract_id = env.register_contract(None, FeeSponsorContract);
        let client = FeeSponsorContractClient::new(&env, &contract_id);

        let sponsor = Address::generate(&env);
        let account = Address::generate(&env);

        // Deposit 1000, sponsor with cap 500
        client.deposit(&sponsor, &1000);
        client.sponsor_account(&sponsor, &account, &500);

        // Draw 300 from first account
        client.draw(&account, &300);
        assert_eq!(client.remaining(&account), 200);

        // Create another account sponsored by same sponsor
        let account2 = Address::generate(&env);
        client.sponsor_account(&sponsor, &account2, &500);

        // Sponsor now has 700 remaining balance (1000 - 300)
        // Try to draw 800 which exceeds both cap (500) and sponsor balance (700)
        // This should fail with ExceedsCap since cap check comes first
        let result = client.try_draw(&account2, &800);
        assert_eq!(result, Err(Ok(Error::ExceedsCap)));

        // Can draw up to cap (500) which is within sponsor balance (700)
        client.draw(&account2, &500);
        assert_eq!(client.remaining(&account2), 0); // Cap 500 - drawn 500 = 0 remaining
    }

    #[test]
    fn test_multiple_partial_draws() {
        let env = Env::default();
        let contract_id = env.register_contract(None, FeeSponsorContract);
        let client = FeeSponsorContractClient::new(&env, &contract_id);

        let sponsor = Address::generate(&env);
        let account = Address::generate(&env);

        client.deposit(&sponsor, &1000);
        client.sponsor_account(&sponsor, &account, &500);

        // Multiple partial draws
        client.draw(&account, &50);
        assert_eq!(client.remaining(&account), 450);

        client.draw(&account, &100);
        assert_eq!(client.remaining(&account), 350);

        client.draw(&account, &25);
        assert_eq!(client.remaining(&account), 325);

        client.draw(&account, &75);
        assert_eq!(client.remaining(&account), 250);

        // Total drawn: 250, remaining: 250
        assert_eq!(client.remaining(&account), 250);
    }

    #[test]
    fn test_invalid_amount() {
        let env = Env::default();
        let contract_id = env.register_contract(None, FeeSponsorContract);
        let client = FeeSponsorContractClient::new(&env, &contract_id);

        let sponsor = Address::generate(&env);

        // Invalid deposit amount
        let result = client.try_deposit(&sponsor, &0);
        assert_eq!(result, Err(Ok(Error::InvalidAmount)));

        let result = client.try_deposit(&sponsor, &-100);
        assert_eq!(result, Err(Ok(Error::InvalidAmount)));
    }

    #[test]
    fn test_insufficient_deposit_for_sponsorship() {
        let env = Env::default();
        let contract_id = env.register_contract(None, FeeSponsorContract);
        let client = FeeSponsorContractClient::new(&env, &contract_id);

        let sponsor = Address::generate(&env);
        let account = Address::generate(&env);

        client.deposit(&sponsor, &100);

        // Try to sponsor with cap exceeding deposit
        let result = client.try_sponsor_account(&sponsor, &account, &200);
        assert_eq!(result, Err(Ok(Error::InsufficientDeposit)));
    }

    #[test]
    fn test_account_not_sponsored() {
        let env = Env::default();
        let contract_id = env.register_contract(None, FeeSponsorContract);
        let client = FeeSponsorContractClient::new(&env, &contract_id);

        let account = Address::generate(&env);

        // Try to draw from non-sponsored account
        let result = client.try_draw(&account, &100);
        assert_eq!(result, Err(Ok(Error::AccountNotSponsored)));

        // Check remaining for non-sponsored account
        assert_eq!(client.remaining(&account), 0);
    }

    #[test]
    fn test_duplicate_sponsorship() {
        let env = Env::default();
        let contract_id = env.register_contract(None, FeeSponsorContract);
        let client = FeeSponsorContractClient::new(&env, &contract_id);

        let sponsor = Address::generate(&env);
        let account = Address::generate(&env);

        client.deposit(&sponsor, &1000);
        client.sponsor_account(&sponsor, &account, &500);

        // Try to sponsor the same account again
        let result = client.try_sponsor_account(&sponsor, &account, &300);
        assert_eq!(result, Err(Ok(Error::AlreadySponsored)));
    }
}
