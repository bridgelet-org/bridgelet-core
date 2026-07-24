#![no_std]

mod authorization;
mod errors;
mod storage;
mod transfers;

mod ephemeral_account_contract {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/ephemeral_account.wasm"
    );
}
use ephemeral_account_contract::Client as EphemeralAccountClient;

use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, IntoVal, Vec,
};

use authorization::AuthContext;
use bridgelet_shared::{AccountStatus, Payment, SweepControllerInterface};
pub use errors::Error;

#[contract]
pub struct SweepController;

#[contractimpl]
impl SweepController {
    /// Initialize the sweep controller with authorized signer
    ///
    /// # Arguments
    /// * `authorized_signer` - Ed25519 public key (32 bytes) that will authorize sweep operations
    /// * `authorized_destination` - Optional destination address. If provided, sweeps can only go to this address (locked mode).
    ///                              If None, any destination is allowed (flexible mode).
    ///
    /// # Errors
    /// Returns Error::AuthorizationFailed if called more than once
    pub fn initialize(
        env: Env,
        creator: Address,
        authorized_signer: BytesN<32>,
        authorized_destination: Option<Address>,
    ) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        // Check if already initialized
        if storage::get_authorized_signer(&env).is_some() {
            return Err(Error::AuthorizationFailed);
        }

        // Require the creator to authorize this initialization
        creator.require_auth();

        storage::set_creator(&env, &creator);

        // Store the authorized signer public key
        storage::set_authorized_signer(&env, &authorized_signer);

        // Initialize the sweep nonce to 0
        storage::init_sweep_nonce(&env);

        // Store authorized destination if provided
        if let Some(destination) = authorized_destination {
            storage::set_authorized_destination(&env, &destination);
            emit_destination_authorized(&env, destination);
        }

        Ok(())
    }

    /// Execute sweep operation from ephemeral account to destination
    ///
    /// # Arguments
    /// * `ephemeral_account` - Address of the ephemeral account contract
    /// * `destination` - Destination wallet address
    /// * `auth_signature` - Authorization signature
    ///
    /// # Errors
    /// Returns Error::AuthorizationFailed if signature is invalid
    /// Returns Error::InvalidAccount if account is not in valid state
    /// Returns Error::TransferFailed if token transfer fails
    /// Returns Error::UnauthorizedDestination if destination doesn't match authorized destination (when set)
    pub fn execute_sweep(
        env: Env,
        ephemeral_account: Address,
        destination: Address,
        auth_signature: BytesN<64>,
    ) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        Self::validate_destination(&env, &destination)?;

        // Verify authorization
        let auth_ctx = AuthContext::new(
            ephemeral_account.clone(),
            destination.clone(),
            auth_signature.clone(),
        );
        auth_ctx.verify(&env)?;

        Self::sweep_account(&env, ephemeral_account, destination, auth_signature, true)
    }

    /// Claim funds to the recipient using Soroban auth entries instead of a
    /// transaction-source signature. This enables a relayer/SDK to submit the
    /// transaction while the recipient only signs the authorization payload.
    pub fn claim(env: Env, recipient: Address, ephemeral_account: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        recipient.require_auth();
        Self::validate_destination(&env, &recipient)?;

        // Read payment info before sweep_claim() changes the account state
        let account_client = EphemeralAccountClient::new(&env, &ephemeral_account);
        let info = account_client.get_info();
        let amount: i128 = info.payments.iter().map(|p| p.amount).sum();

        Self::authorize_claim(&env, &ephemeral_account, &recipient)?;
        emit_sweep_completed(&env, ephemeral_account, recipient, amount);

        Ok(())
    }

    fn validate_destination(env: &Env, destination: &Address) -> Result<(), Error> {
        if storage::has_authorized_destination(env) {
            let authorized_dest =
                storage::get_authorized_destination(env).ok_or(Error::UnauthorizedDestination)?;
            if *destination != authorized_dest {
                return Err(Error::UnauthorizedDestination);
            }
        }

        Ok(())
    }

    fn authorize_ephemeral_sweep(
        env: &Env,
        ephemeral_account: &Address,
        destination: &Address,
        auth_signature: &BytesN<64>,
    ) {
        let args = (destination.clone(), auth_signature.clone()).into_val(env);
        let context = ContractContext {
            contract: ephemeral_account.clone(),
            fn_name: symbol_short!("sweep"),
            args,
        };
        let auth_entries = Vec::from_array(
            env,
            [InvokerContractAuthEntry::Contract(SubContractInvocation {
                context,
                sub_invocations: Vec::new(env),
            })],
        );
        env.authorize_as_current_contract(auth_entries);
    }

    fn sweep_account(
        env: &Env,
        ephemeral_account: Address,
        destination: Address,
        auth_signature: BytesN<64>,
        increment_nonce: bool,
    ) -> Result<(), Error> {
        if increment_nonce {
            // Increment nonce after successful verification to prevent replay attacks.
            authorization::increment_nonce(env);
        }

        Self::authorize_ephemeral_sweep(env, &ephemeral_account, &destination, &auth_signature);

        // Call ephemeral account contract to validate and authorize sweep.
        let account_client = EphemeralAccountClient::new(env, &ephemeral_account);

        // The account contract validates state and authorizes the sweep.
        account_client.sweep(&destination, &auth_signature);

        // Get payment details from account.
        let info = account_client.get_info();

        // Verify payment was received
        if !info.payment_received {
            return Err(Error::AccountNotReady);
        }

        let amount = info.payments.iter().map(|p| p.amount).sum();
        if amount == 0 {
            return Err(Error::AccountNotReady);
        }

        // Execute the actual token transfers for all recorded payments.
        //
        // info.payments yields ephemeral_account_contract::Payment (the
        // contractimport!-generated type) — structurally identical to
        // bridgelet_shared::Payment but a distinct Rust type, since
        // contractimport! derives its own types from the wasm's interface
        // metadata rather than reusing the shared crate. Convert explicitly
        // field-by-field; transfers::execute_transfers expects the
        // bridgelet_shared version.
        let mut payments_vec = Vec::new(env);
        for payment in info.payments.iter() {
            payments_vec.push_back(Payment {
                asset: payment.asset.clone(),
                amount: payment.amount,
                timestamp: payment.timestamp,
            });
        }

        transfers::execute_transfers(env, &ephemeral_account, &destination, &payments_vec)
            .map_err(|_| Error::TransferFailed)?;

        // Emit sweep completed event after successful transfer.
        emit_sweep_completed(env, ephemeral_account, destination, amount);

        Ok(())
    }

    // Replace the entire authorize_claim function:
    fn authorize_claim(
        env: &Env,
        ephemeral_account: &Address,
        recipient: &Address,
    ) -> Result<(), Error> {
        // Authorize the controller as the invoker of sweep_claim on the ephemeral account
        let args = (recipient.clone(),).into_val(env);
        let context = ContractContext {
            contract: ephemeral_account.clone(),
            fn_name: symbol_short!("swp_claim"), // symbol_short! max 9 chars — abbreviated
            args,
        };
        let auth_entries = Vec::from_array(
            env,
            [InvokerContractAuthEntry::Contract(SubContractInvocation {
                context,
                sub_invocations: Vec::new(env),
            })],
        );
        env.authorize_as_current_contract(auth_entries);

        let account_client = EphemeralAccountClient::new(env, ephemeral_account);
        account_client.sweep_claim(recipient);
        Ok(())
    }
    /// Check if an account is ready for sweep
    pub fn can_sweep(env: Env, ephemeral_account: Address) -> bool {
        storage::extend_instance_ttl(&env);

        let account_client = EphemeralAccountClient::new(&env, &ephemeral_account);

        let info = account_client.get_info();

        info.status as u32 == AccountStatus::PaymentReceived as u32
            && !account_client.is_expired()
    }

    /// Return the current sweep nonce for this controller.
    ///
    /// Off-chain signers must sign a `construct_sweep_message()` payload
    /// built with THIS value, not a locally-tracked guess — the contract
    /// always verifies against its own current on-chain nonce, so a stale
    /// or mistracked nonce here produces a signature that will not verify.
    /// Starts at 0 at `initialize()` and increments by 1 after every
    /// successful `execute_sweep()`/`claim()` call.
    pub fn get_nonce(env: Env) -> u64 {
        storage::extend_instance_ttl(&env);

        storage::get_sweep_nonce(&env)
    }

    /// Update the authorized destination address
    ///
    /// This function allows the creator to update the authorized destination before any sweep occurs.
    /// Once a sweep has been executed, the destination cannot be changed.
    ///
    /// # Arguments
    /// * `new_destination` - New authorized destination address
    ///
    /// # Errors
    /// Returns Error::AuthorizationFailed if caller is not the creator
    /// Returns Error::AccountAlreadySwept if a sweep has already been executed
    pub fn update_authorized_destination(env: Env, new_destination: Address) -> Result<(), Error> {
        storage::extend_instance_ttl(&env);

        // Verify creator authorization
        let creator = storage::get_creator(&env).ok_or(Error::AuthorizationFailed)?;
        creator.require_auth();

        // Check if a sweep has already been executed (nonce > 0 indicates at least one sweep)
        let nonce = storage::get_sweep_nonce(&env);
        if nonce > 0 {
            return Err(Error::AccountAlreadySwept);
        }

        // Update the authorized destination
        let old_destination = storage::get_authorized_destination(&env);
        storage::set_authorized_destination(&env, &new_destination);

        // Emit event
        emit_destination_updated(&env, old_destination, new_destination);

        Ok(())
    }
}

/// Issue #43: conform to the shared interface for type-safe SDK integration.
/// Each method delegates to the inherent contract implementation above.
impl SweepControllerInterface for SweepController {
    type Error = Error;

    fn initialize(
        env: Env,
        creator: Address,
        authorized_signer: BytesN<32>,
        authorized_destination: Option<Address>,
    ) -> Result<(), Error> {
        Self::initialize(env, creator, authorized_signer, authorized_destination)
    }

    fn execute_sweep(
        env: Env,
        ephemeral_account: Address,
        destination: Address,
        auth_signature: BytesN<64>,
    ) -> Result<(), Error> {
        Self::execute_sweep(env, ephemeral_account, destination, auth_signature)
    }

    fn claim(env: Env, recipient: Address, ephemeral_account: Address) -> Result<(), Error> {
        Self::claim(env, recipient, ephemeral_account)
    }
}

/// Sweep completed event
#[contracttype]
#[derive(Clone, Debug)]
pub struct SweepCompleted {
    pub ephemeral_account: Address,
    pub destination: Address,
    pub amount: i128,
}

/// Destination authorized event (emitted when destination is set during initialization)
#[contracttype]
#[derive(Clone, Debug)]
pub struct DestinationAuthorized {
    pub destination: Address,
}

/// Destination updated event (emitted when authorized destination is updated)
#[contracttype]
#[derive(Clone, Debug)]
pub struct DestinationUpdated {
    pub old_destination: Option<Address>,
    pub new_destination: Address,
}

fn emit_sweep_completed(env: &Env, account: Address, destination: Address, amount: i128) {
    let event = SweepCompleted {
        ephemeral_account: account,
        destination,
        amount,
    };
    env.events()
        .publish((soroban_sdk::symbol_short!("sweep"),), event);
}

fn emit_destination_authorized(env: &Env, destination: Address) {
    let event = DestinationAuthorized { destination };
    env.events()
        .publish((soroban_sdk::symbol_short!("dest_auth"),), event);
}

fn emit_destination_updated(env: &Env, old_destination: Option<Address>, new_destination: Address) {
    let event = DestinationUpdated {
        old_destination,
        new_destination,
    };
    env.events()
        .publish((soroban_sdk::symbol_short!("dest_upd"),), event);
}
