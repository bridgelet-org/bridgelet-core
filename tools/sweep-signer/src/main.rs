//! sweep-signer
//!
//! Two things you need for AUTHORIZED_SIGNER_PUBLIC_KEY / sweep signing:
//!
//!   1. Before deploying: derive the PUBLIC key from your signing seed, to
//!      put in AUTHORIZED_SIGNER_PUBLIC_KEY / pass to SweepController::initialize().
//!         sweep-signer pubkey --signer-seed-hex <64 hex chars>
//!
//!   2. Per sweep, once deployed: produce the signature for execute_sweep().
//!         sweep-signer sign --contract-id ... --destination ... --nonce ... --signer-seed-hex ...
//!
//! Message format (matches contracts/sweep_controller/src/authorization.rs
//! exactly - NOT the timestamp-including format that was in the old
//! docs/SIGNATURE_FORMAT.md before it was corrected):
//!
//!   message = SHA256( destination.to_xdr() || nonce_be_u64(8 bytes) || contract_id.to_xdr() )
//!   signature = Ed25519_sign(message, signer_private_key)
//!
//! Accepts the signing key as EITHER:
//!   --signer-seed-hex <64 hex chars>   raw 32-byte Ed25519 seed (e.g. from
//!                                      `node -e "console.log(require('crypto').randomBytes(32).toString('hex'))"`)
//!   --signer-secret   <S... string>    a Stellar strkey secret key
//! This key is signing-only - it never needs to be a funded Stellar account,
//! so the raw hex form is the simpler option and is recommended.
//!
//! Address XDR encoding uses soroban_sdk::Address::to_xdr() itself (via a
//! throwaway local Env, no network involved), so serialization is
//! guaranteed to match what the deployed contract computes on-chain rather
//! than relying on a hand-rolled XDR encoder.

use clap::{Args, Parser, Subcommand};
use ed25519_dalek::{Signer, SigningKey};
use soroban_sdk::{xdr::ToXdr, Address, Bytes, Env};

#[derive(Parser)]
#[command(about = "Key derivation and signing for bridgelet-core's SweepController")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Derive the public key to register as AUTHORIZED_SIGNER_PUBLIC_KEY /
    /// pass to SweepController::initialize(). Run this BEFORE deploying.
    Pubkey(KeyArgs),

    /// Produce the auth_signature for a specific sweep. Run this AFTER
    /// deploying, once you have a real contract_id and current nonce.
    Sign(SignArgs),
}

#[derive(Args)]
struct KeyArgs {
    #[command(flatten)]
    key: SignerKey,
}

#[derive(Args)]
struct SignArgs {
    /// SweepController contract ID (C... address)
    #[arg(long)]
    contract_id: String,

    /// Destination wallet address funds will be swept to (G... address)
    #[arg(long)]
    destination: String,

    /// Current sweep nonce for this SweepController deployment. Query it
    /// with SweepController::get_nonce() on the live contract immediately
    /// before signing - do not track/guess this locally, it will drift.
    #[arg(long)]
    nonce: u64,

    #[command(flatten)]
    key: SignerKey,
}

#[derive(Args)]
#[group(required = true, multiple = false)]
struct SignerKey {
    /// Raw 32-byte Ed25519 seed, hex-encoded (64 hex chars). Recommended -
    /// this key is signing-only and never needs to be a funded Stellar
    /// account.
    #[arg(long, env = "SWEEP_SIGNING_KEY_SEED")]
    signer_seed_hex: Option<String>,

    /// Stellar secret key in S... strkey format, if you'd rather manage
    /// this as a normal Stellar keypair (e.g. via `stellar keys generate`).
    #[arg(long, env = "AUTHORIZED_SIGNER_SECRET")]
    signer_secret: Option<String>,
}

impl SignerKey {
    fn to_signing_key(&self) -> SigningKey {
        let seed: [u8; 32] = if let Some(hex_seed) = &self.signer_seed_hex {
            let bytes = hex::decode(hex_seed).unwrap_or_else(|e| {
                eprintln!("Invalid --signer-seed-hex: {e}");
                std::process::exit(1);
            });
            bytes.try_into().unwrap_or_else(|v: Vec<u8>| {
                eprintln!(
                    "--signer-seed-hex must decode to exactly 32 bytes, got {}",
                    v.len()
                );
                std::process::exit(1);
            })
        } else if let Some(secret) = &self.signer_secret {
            match stellar_strkey::ed25519::PrivateKey::from_string(secret) {
                Ok(pk) => pk.0,
                Err(e) => {
                    eprintln!("Invalid --signer-secret: {e}");
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("Provide either --signer-seed-hex or --signer-secret");
            std::process::exit(1);
        };

        SigningKey::from_bytes(&seed)
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Pubkey(args) => {
            let signing_key = args.key.to_signing_key();
            let pubkey_hex = hex::encode(signing_key.verifying_key().to_bytes());
            println!("Public key (hex) - put this in AUTHORIZED_SIGNER_PUBLIC_KEY:");
            println!("{pubkey_hex}");
        }

        Command::Sign(args) => {
            let signing_key = args.key.to_signing_key();

            // Local, network-free Env - used only to get soroban_sdk's own
            // Address::to_xdr() encoding, guaranteed to match on-chain.
            let env = Env::default();

            let destination = Address::from_str(&env, &args.destination);
            let contract_id = Address::from_str(&env, &args.contract_id);

            let mut message = Bytes::new(&env);
            message.append(&destination.to_xdr(&env));

            for shift in (0..8).rev() {
                message.push_back(((args.nonce >> (shift * 8)) & 0xFF) as u8);
            }

            message.append(&contract_id.to_xdr(&env));

            let digest: soroban_sdk::BytesN<32> = env.crypto().sha256(&message).into();
            let mut digest_bytes = [0u8; 32];
            digest.copy_into_slice(&mut digest_bytes);

            let signature = signing_key.sign(&digest_bytes);

            println!(
                "auth_signature (hex, pass to execute_sweep): {}",
                hex::encode(signature.to_bytes())
            );
            println!(
                "signer public key (hex, sanity-check against AUTHORIZED_SIGNER_PUBLIC_KEY): {}",
                hex::encode(signing_key.verifying_key().to_bytes())
            );
            println!();
            println!(
                "⚠️  Nonce used: {}. Confirm this matches SweepController::get_nonce() on the",
                args.nonce
            );
            println!("   live contract at the moment you sign - a stale nonce produces a");
            println!("   signature the contract will reject.");
        }
    }
}