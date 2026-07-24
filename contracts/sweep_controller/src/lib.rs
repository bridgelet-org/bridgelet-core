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
use bridgelet_shared::{AccountStatus, Payment};
pub use errors::Error;

/// Number of ledgers for the signer update time-lock (48 hours).
/// Stellar ledgers close approximately every 5 seconds:
/// 48 hours * 60 min * 60 sec / 5 sec = 34,560 ledgers.
const SIGNER_TIMELOCK_LEDGERS: u32 = 34_560;

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
        let account_client = EphemeralAccountClient::new(&env, &ephemeral_account);

        // Check if account exists and has payment
        let info = account_client.get_info();

        info.payment_received
            && info.status as u32 == AccountStatus::PaymentReceived as u32
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

    /// Estimate the fee for a sweep operation.
    ///
    /// Returns `(estimated_fee, total_amount)` where:
    /// - `estimated_fee`: always 0 on Soroban (no gas fees for contract
    ///   invocations beyond the transaction fee paid by the submitter).
    /// - `total_amount`: sum of all recorded payment amounts for the
    ///   given ephemeral account.
    ///
    /// This is a read-only view — makes no state changes.  Useful for the
    /// SDK to display expected amounts to senders before account creation.
    ///
    /// # Errors
    /// Returns `Error::InvalidAccount` if the ephemeral account cannot be queried
    /// Returns `Error::AccountNotReady` if no payment has been recorded
    pub fn fee_estimate(env: Env, ephemeral_account: Address) -> Result<(i128, i128), Error> {
        let account_client = EphemeralAccountClient::new(&env, &ephemeral_account);

        if !account_client.is_initialized() {
            return Err(Error::InvalidAccount);
        }

        let info = account_client.get_info();

        if !info.payment_received || info.payments.is_empty() {
            return Err(Error::AccountNotReady);
        }

        let total_amount: i128 = info.payments.iter().map(|p| p.amount).sum();

        // Soroban does not charge per-invocation gas fees — the transaction
        // fee is paid by the submitter.  We return 0 to signal "no
        // additional fee" to the SDK.
        Ok((0, total_amount))
    }

    /// Initiate a time-locked update of the authorized signer.
    ///
    /// The new signer will only take effect after `SIGNER_TIMELOCK_LEDGERS`
    /// (48 hours worth of ledgers) have passed since this call.  This
    /// prevents hasty key changes and gives the operator time to detect
    /// and cancel a compromised key rotation.
    ///
    /// Only the creator/admin can call this function.
    ///
    /// # Arguments
    /// * `new_signer` - Ed25519 public key of the new authorized signer
    ///
    /// # Errors
    /// Returns `Error::NotAdmin` if caller is not the creator
    pub fn update_authorized_signer(env: Env, new_signer: BytesN<32>) -> Result<(), Error> {
        let creator = storage::get_creator(&env).ok_or(Error::NotAdmin)?;
        creator.require_auth();

        let current_ledger = env.ledger().sequence();
        let effective_ledger = current_ledger
            .checked_add(SIGNER_TIMELOCK_LEDGERS)
            .ok_or(Error::Overflow)?;

        storage::set_pending_signer(&env, &new_signer);
        storage::set_pending_signer_effective_ledger(&env, effective_ledger);

        emit_signer_update_initiated(&env, new_signer, effective_ledger);

        Ok(())
    }

    /// Apply a pending signer update after the time-lock has elapsed.
    ///
    /// Anyone can call this once the effective ledger has been reached.
    /// The pending signer is cleared after application.
    ///
    /// # Errors
    /// Returns `Error::NoPendingSignerUpdate` if no update was initiated
    /// Returns `Error::TimeLockNotElapsed` if the effective ledger has not passed
    pub fn apply_signer_update(env: Env) -> Result<(), Error> {
        let pending = storage::get_pending_signer(&env).ok_or(Error::NoPendingSignerUpdate)?;
        let effective = storage::get_pending_signer_effective_ledger(&env)
            .ok_or(Error::NoPendingSignerUpdate)?;

        let current_ledger = env.ledger().sequence();
        if current_ledger < effective {
            return Err(Error::TimeLockNotElapsed);
        }

        storage::set_authorized_signer(&env, &pending);
        storage::clear_pending_signer(&env);

        emit_signer_update_applied(&env, pending);

        Ok(())
    }

    /// View the pending signer update state.
    ///
    /// Returns `(pending_signer, effective_ledger)` or `None` if no update
    /// is pending.
    pub fn get_pending_signer_update(env: Env) -> Option<(BytesN<32>, u32)> {
        let pending = storage::get_pending_signer(&env)?;
        let effective = storage::get_pending_signer_effective_ledger(&env)?;
        Some((pending, effective))
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

/// Emitted when a time-locked signer update is initiated
#[contracttype]
#[derive(Clone, Debug)]
pub struct SignerUpdateInitiated {
    pub new_signer: BytesN<32>,
    pub effective_ledger: u32,
}

/// Emitted when a pending signer update is applied
#[contracttype]
#[derive(Clone, Debug)]
pub struct SignerUpdateApplied {
    pub new_signer: BytesN<32>,
}

fn emit_signer_update_initiated(env: &Env, new_signer: BytesN<32>, effective_ledger: u32) {
    let event = SignerUpdateInitiated {
        new_signer,
        effective_ledger,
    };
    env.events()
        .publish((soroban_sdk::symbol_short!("sig_init"),), event);
}

fn emit_signer_update_applied(env: &Env, new_signer: BytesN<32>) {
    let event = SignerUpdateApplied { new_signer };
    env.events()
        .publish((soroban_sdk::symbol_short!("sig_appl"),), event);
}
