use soroban_sdk::{Bytes, BytesN, Env};

pub const PUBLIC_NETWORK_PASSPHRASE: &str = "Public Global Stellar Network ; September 2015";
pub const TESTNET_PASSPHRASE: &str = "Test SDF Network ; September 2015";
pub const STANDALONE_PASSPHRASE: &str = "Standalone Network ; February 2017";

fn hash_passphrase(env: &Env, passphrase: &str) -> BytesN<32> {
    let bytes = Bytes::from_slice(env, passphrase.as_bytes());
    env.crypto().sha256(&bytes)
}

pub fn require_network(env: &Env, expected_passphrase: &str) -> Result<(), BytesN<32>> {
    let actual = env.ledger().network_id();
    let expected = hash_passphrase(env, expected_passphrase);
    if actual == expected {
        Ok(())
    } else {
        Err(expected)
    }
}
