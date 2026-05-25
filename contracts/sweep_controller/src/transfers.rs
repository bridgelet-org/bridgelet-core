use crate::errors::Error;
use soroban_sdk::token::TokenClient;
use soroban_sdk::{Address, Env};

/// Execute token transfer from ephemeral account to destination
pub fn execute_transfer(
    env: &Env,
    token_address: &Address,
    from: &Address,
    to: &Address,
    amount: i128,
) -> Result<(), Error> {
    // Create token client
    let token = TokenClient::new(env, token_address);

    // Execute transfer
    token.transfer(from, to, &amount);

    Ok(())
}

/// Transfer context for sweep operations
pub struct TransferContext {
    pub asset: Address,
    pub from: Address,
    pub to: Address,
    pub amount: i128,
}

impl TransferContext {
    pub fn new(asset: Address, from: Address, to: Address, amount: i128) -> Self {
        Self {
            asset,
            from,
            to,
            amount,
        }
    }

    pub fn execute(&self, env: &Env) -> Result<(), Error> {
        execute_transfer(env, &self.asset, &self.from, &self.to, self.amount)
    }
}
