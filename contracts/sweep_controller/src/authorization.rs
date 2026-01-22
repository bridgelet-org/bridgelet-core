use crate::errors::Error;
use soroban_sdk::{Address, BytesN, Env};

/// Verify sweep authorization signature
pub fn verify_sweep_auth(
    _env: &Env,
    _account: &Address,
    _destination: &Address,
    _signature: &BytesN<64>,
) -> Result<(), Error> {
    // TODO: Implement proper signature verification
    // For MVP, trust the caller to provide valid signatures
    // Future: Verify Ed25519 signature against authorized signer
    Ok(())
}

/// Authorization context for sweep operations
pub struct AuthContext {
    pub account: Address,
    pub destination: Address,
    pub signature: BytesN<64>,
}

impl AuthContext {
    pub fn new(account: Address, destination: Address, signature: BytesN<64>) -> Self {
        Self {
            account,
            destination,
            signature,
        }
    }

    pub fn verify(&self, env: &Env) -> Result<(), Error> {
        verify_sweep_auth(env, &self.account, &self.destination, &self.signature)
    }
}
