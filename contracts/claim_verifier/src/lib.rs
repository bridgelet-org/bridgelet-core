#![no_std]

mod errors;
mod events;

use soroban_sdk::{contract, contractimpl, Address, Bytes, BytesN, Env};

pub use errors::Error;
pub use events::VerificationSucceeded;

#[contract]
pub struct ClaimVerifierContract;

#[contractimpl]
impl ClaimVerifierContract {
    /// Initialize with the authorized signer public key
    pub fn initialize(env: Env, authorized_signer: BytesN<32>) -> Result<(), Error> {
        if env.storage().instance().has(&"signer") {
            return Err(Error::AlreadyInitialized);
        }

        env.storage().instance().set(&"signer", &authorized_signer);

        Ok(())
    }

    /// Verify an Ed25519 sweep authorization signature.
    /// Message format: hash(destination + nonce + contract_id)
    pub fn verify(
        env: Env,
        destination: Address,
        nonce: u64,
        signature: BytesN<64>,
    ) -> Result<(), Error> {
        let authorized_signer: BytesN<32> = env
            .storage()
            .instance()
            .get(&"signer")
            .ok_or(Error::AuthorizedSignerNotSet)?;

        let message = Self::construct_message(&env, &destination, nonce);

        env.crypto()
            .ed25519_verify(&authorized_signer, &message.clone().into(), &signature);

        events::emit_verification_succeeded(&env, destination, nonce);

        Ok(())
    }

    fn construct_message(env: &Env, destination: &Address, nonce: u64) -> BytesN<32> {
        use soroban_sdk::xdr::ToXdr;

        let contract_id = env.current_contract_address();
        let mut message = Bytes::new(env);

        message.append(&destination.to_xdr(env));
        message.push_back(((nonce >> 56) & 0xFF) as u8);
        message.push_back(((nonce >> 48) & 0xFF) as u8);
        message.push_back(((nonce >> 40) & 0xFF) as u8);
        message.push_back(((nonce >> 32) & 0xFF) as u8);
        message.push_back(((nonce >> 24) & 0xFF) as u8);
        message.push_back(((nonce >> 16) & 0xFF) as u8);
        message.push_back(((nonce >> 8) & 0xFF) as u8);
        message.push_back((nonce & 0xFF) as u8);
        message.append(&contract_id.to_xdr(env));

        env.crypto().sha256(&message).into()
    }
}
