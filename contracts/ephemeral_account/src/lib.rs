#![no_std]
#![allow(warnings)]

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod test;

pub use bridgelet_shared::{AccountInfo, AccountStatus, Payment};
pub use errors::Error;
pub use events::{
    AccountCreated, AccountExpired, MultiPaymentReceived, PaymentReceived, ReserveReclaimed,
    SweepExecutedMulti,
};
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Vec};
pub use storage::DataKey;

use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env, Vec};
const BASE_RESERVE_STROOPS: i128 = 1_000_000_000;
const CONTRACT_VERSION: u32 = 1;
#[contract]
pub struct EphemeralAccountContract;
#[contractimpl]
impl EphemeralAccountContract {
    /// Initialize the ephemeral account with restrictions
    ///
    /// # Arguments
    /// * `creator` - Address that created this account (must sign this call)
    /// * `expiry_ledger` - Ledger number when account expires
    /// * `recovery_address` - Address to return funds if expired
    /// * `authorized_signer` - Ed25519 public key (32 bytes) whose signatures authorize sweeps
    ///
    /// # Errors
    /// Returns Error::AlreadyInitialized if called more than once
    pub fn initialize(
        env: Env,
        creator: Address,
        expiry_ledger: u32,
        recovery_address: Address,
        authorized_controller: Address,
        relayer: Address,
        authorized_signer: BytesN<32>,
        min_amount: i128,
    ) -> Result<(), Error> {
        if storage::is_initialized(&env) {
            return Err(Error::AlreadyInitialized);
        }
        creator.require_auth();
        let current_ledger = env.ledger().sequence();
        if expiry_ledger <= current_ledger {
            return Err(Error::InvalidExpiry);
        }

        // Validate min_amount is non-negative
        if min_amount < 0 {
            return Err(Error::InvalidAmount);
        }

        // Store initialization data
        storage::set_initialized(&env, true);
        storage::set_creator(&env, &creator);
        storage::set_expiry_ledger(&env, expiry_ledger);
        storage::set_recovery_address(&env, &recovery_address);
        storage::set_status(&env, AccountStatus::Active);
        storage::set_authorized_controller(&env, &authorized_controller);
        storage::set_relayer(&env, &relayer);
        storage::set_authorized_signer(&env, &authorized_signer);
        storage::set_min_payment_amount(&env, min_amount);
        storage::init_reserve_tracking(&env, BASE_RESERVE_STROOPS);
        storage::set_contract_version(&env, CONTRACT_VERSION);
        events::emit_account_created(&env, creator, expiry_ledger);
        Ok(())
    }

    /// Record an inbound payment to this ephemeral account.
    /// Multiple payments with different assets are supported.
    ///
    /// # Arguments
    /// * `amount` - Payment amount (must be positive)
    /// * `asset` - Asset contract address
    ///
    /// # Errors
    /// * [`Error::NotInitialized`] – account not yet initialised
    /// * [`Error::InvalidAmount`] – amount is not positive
    /// * [`Error::PaymentAlreadyReceived`] – a payment was already recorded
    pub fn record_payment(env: Env, amount: i128, asset: Address) -> Result<(), Error> {
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        // Enforce single inbound payment restriction
        if storage::has_payment_received(&env) {
            return Err(Error::PaymentAlreadyReceived);
        }

        // Check minimum payment amount
        let min_amount = storage::get_min_payment_amount(&env);
        if amount < min_amount {
            return Err(Error::PaymentBelowMinimum);
        }

        // Check for duplicate asset
        if storage::get_payment(&env, &asset).is_some() {
            return Err(Error::DuplicateAsset);
        }
        let payment_count = storage::get_total_payments(&env);
        if payment_count >= 10 {
            return Err(Error::TooManyPayments);
        }
        let payment = Payment {
            asset: asset.clone(),
            amount,
            timestamp: env.ledger().timestamp(),
        };
        storage::add_payment(&env, payment);
        if payment_count == 0 {
            storage::set_status(&env, AccountStatus::PaymentReceived);
        }
        if payment_count == 0 {
            events::emit_payment_received(&env, amount, asset);
        } else {
            events::emit_multi_payment_received(&env, asset, amount);
        }
        Ok(())
    }

    /// Execute sweep to destination wallet.
    ///
    /// Transfers all recorded asset balances from this contract to `destination`
    /// using the Soroban [`token::TokenClient`] interface. The status is updated
    /// to [`AccountStatus::Swept`] before the transfers to prevent re-entrancy;
    /// any token call that panics will revert the entire transaction.
    ///
    /// # Arguments
    /// * `destination` - Recipient wallet address
    /// * `auth_signature` - Authorization signature from off-chain system
    ///
    /// # Errors
    /// * [`Error::AlreadySwept`] — sweep has already been executed
    /// * [`Error::NoPaymentReceived`] — no payment recorded yet
    /// * [`Error::AccountExpired`] — account has passed its expiry ledger
    /// * [`Error::Unauthorized`] — authorization check failed
    pub fn sweep(env: Env, destination: Address, auth_signature: BytesN<64>) -> Result<(), Error> {
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }
        if storage::get_status(&env) == AccountStatus::Swept {
            return Err(Error::AlreadySwept);
        }
        if !storage::has_payment_received(&env) {
            return Err(Error::NoPaymentReceived);
        }
        if Self::is_expired(env.clone()) {
            return Err(Error::AccountExpired);
        }
        // Verify authorization signature
        // Note: In production, implement proper signature verification
        // For MVP, we trust the SDK to only call with valid signatures
        // Enforce single-destination lock.
        if let Some(locked) = storage::get_sweep_destination(&env) {
            if locked != destination {
                return Err(Error::SweepDestinationLocked);
            }
        } else {
            storage::set_sweep_destination(&env, &destination);
        }
        Self::verify_sweep_authorization(&env, &destination, &auth_signature)?;
        let payments = storage::get_all_payments(&env);
        let mut payments_vec = Vec::new(&env);
        for payment in payments.values() {
            payments_vec.push_back(payment);
        }
        // Update status before transfers to prevent re-entrancy.
        storage::set_status(&env, AccountStatus::Swept);
        storage::set_swept_to(&env, &destination);

        // Note: Actual token transfers are executed by the SweepController via SEP-0010 / Stellar SDK.
        // This contract enforces authorization/state transitions and reserve lifecycle.
        let sweep_id = env.ledger().sequence() as u64;
        storage::set_last_sweep_id(&env, sweep_id);
        // Execute on-chain token transfers for every recorded payment.
        let contract_address = env.current_contract_address();
        for payment in payments_vec.iter() {
            token::TokenClient::new(&env, &payment.asset).transfer(
                &contract_address,
                &destination,
                &payment.amount,
            );
        }
        events::emit_sweep_executed_multi(&env, destination.clone(), &payments_vec);
        Self::reclaim_reserve_to(&env, &destination, sweep_id)?;
        Ok(())
    }

    /// Check if the account has expired.
    ///
    /// Expiry is determined by comparing `env.ledger().sequence()` against
    /// `expiry_ledger`. Returns `false` if the account is not initialized.
    /// Check whether this ephemeral account has expired.
    ///
    /// ## Ledger time vs wall-clock time
    ///
    /// Soroban smart contracts cannot safely rely on wall-clock (UNIX) time for
    /// consensus-critical comparisons because `env.ledger().timestamp()` reflects
    /// the timestamp set by the validator and can drift slightly between ledgers.
    /// Instead, expiry is tracked using the **ledger sequence number**
    /// (`env.ledger().sequence()`), which increments by exactly 1 per closed
    /// ledger and is the canonical, manipulation-resistant clock on Stellar.
    ///
    /// The `expiry_ledger` stored at initialization represents the first ledger
    /// at which the account is considered expired. On Stellar mainnet each ledger
    /// closes approximately every 5 seconds, so the relationship between ledger
    /// ticks and wall-clock duration is:
    ///
    /// ```text
    /// expiry_ledger = current_ledger + (desired_duration_seconds / ~5)
    /// ```
    ///
    /// Returns `false` if the account has not yet been initialized.
    pub fn is_expired(env: Env) -> bool {
        if !storage::is_initialized(&env) {
            return false;
        }
        let expiry_ledger = storage::get_expiry_ledger(&env);
        let current_ledger = env.ledger().sequence();
        current_ledger >= expiry_ledger
    }

    /// Get the contract version stored at initialization.
    pub fn version(env: Env) -> u32 {
        storage::get_contract_version(&env)
    }

    /// Get current account status.
    pub fn get_status(env: Env) -> AccountStatus {
        if !storage::is_initialized(&env) {
            return AccountStatus::Active;
        }
        storage::get_status(&env)
    }

    /// Expire the account and return funds to recovery address.
    ///
    /// # Errors
    /// Returns Error::NotExpired if called before expiry ledger
    pub fn expire(env: Env) -> Result<(), Error> {
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }
        let status = storage::get_status(&env);
        if status == AccountStatus::Swept || status == AccountStatus::Expired {
            return Err(Error::InvalidStatus);
        }
        if !Self::is_expired(env.clone()) {
            return Err(Error::NotExpired);
        }
        let recovery_address = storage::get_recovery_address(&env);
        storage::set_status(&env, AccountStatus::Expired);
        storage::set_swept_to(&env, &recovery_address);
        let total_amount = if storage::has_payment_received(&env) {
            let payments = storage::get_all_payments(&env);
            let mut total = 0i128;
            for (_, payment) in payments.iter() {
                total = total
                    .checked_add(payment.amount)
                    .ok_or(Error::InvalidAmount)?;
            }
            total
        } else {
            0
        };
        let sweep_id = env.ledger().sequence() as u64;
        storage::set_last_sweep_id(&env, sweep_id);
        let reclaimed_reserve = Self::reclaim_reserve_to(&env, &recovery_address, sweep_id)?;
        events::emit_account_expired(&env, recovery_address, total_amount, reclaimed_reserve);
        Ok(())
    }

    /// Reclaim remaining base reserve for a previously swept/expired account.
    pub fn reclaim_reserve(env: Env) -> Result<i128, Error> {
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }
        let status = storage::get_status(&env);
        if status != AccountStatus::Swept && status != AccountStatus::Expired {
            return Err(Error::InvalidStatus);
        }
        let destination = storage::get_swept_to(&env).ok_or(Error::InvalidStatus)?;
        let sweep_id = storage::get_last_sweep_id(&env);
        Self::reclaim_reserve_to(&env, &destination, sweep_id)
    }

    /// Remaining reserve amount (stroops) still eligible for reclaim.
    pub fn get_reserve_remaining(env: Env) -> i128 {
        if !storage::is_initialized(&env) {
            return 0;
        }
        storage::get_base_reserve_remaining(&env)
    }

    /// Tracked reserve currently available for transfer (stroops).
    pub fn get_reserve_available(env: Env) -> i128 {
        if !storage::is_initialized(&env) {
            return 0;
        }
        storage::get_available_reserve(&env)
    }

    /// Whether reserve has been fully reclaimed.
    pub fn is_reserve_reclaimed(env: Env) -> bool {
        if !storage::is_initialized(&env) {
            return false;
        }
        storage::is_reserve_reclaimed(&env)
    }

    /// Last reserve reclaim event payload emitted by this contract.
    pub fn get_last_reserve_event(env: Env) -> Option<ReserveReclaimed> {
        if !storage::is_initialized(&env) {
            return None;
        }
        storage::get_last_reserve_event(&env)
    }

    /// Number of reserve reclaim events emitted by this contract.
    pub fn get_reserve_reclaim_event_count(env: Env) -> u32 {
        if !storage::is_initialized(&env) {
            return 0;
        }
        storage::get_reserve_event_count(&env)
    }

    /// Get account information.
    pub fn get_info(env: Env) -> Result<AccountInfo, Error> {
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }
        let payments = storage::get_all_payments(&env);
        let payment_count = payments.len();
        Ok(AccountInfo {
            creator: storage::get_creator(&env),
            status: storage::get_status(&env),
            expiry_ledger: storage::get_expiry_ledger(&env),
            recovery_address: storage::get_recovery_address(&env),
            payment_received: payment_count > 0,
            payment_count,
            payments: {
                let mut payments_vec = Vec::new(&env);
                for payment in payments.values() {
                    payments_vec.push_back(payment);
                }
                payments_vec
            },
            swept_to: storage::get_swept_to(&env),
        })
    }

    fn verify_sweep_authorization(
        env: &Env,
        destination: &Address,
        signature: &BytesN<64>,
    ) -> Result<(), Error> {
        let signer = storage::get_authorized_signer(env).ok_or(Error::Unauthorized)?;

        // Construct deterministic message: sha256(destination_xdr || contract_id_xdr)
        let mut msg = soroban_sdk::Bytes::new(env);
        msg.append(&destination.to_xdr(env));
        msg.append(&env.current_contract_address().to_xdr(env));
        let message_hash: BytesN<32> = env.crypto().sha256(&msg).into();

        // Panics (and reverts the tx) if the signature is invalid — this is the
        // correct Soroban pattern; no need to wrap in a separate error branch.
        env.crypto()
            .ed25519_verify(&signer, &message_hash.into(), signature);

        Ok(())
    }

    fn reclaim_reserve_to(env: &Env, destination: &Address, sweep_id: u64) -> Result<i128, Error> {
        let reserve_remaining = storage::get_base_reserve_remaining(env);
        let reserve_available = storage::get_available_reserve(env);
        if reserve_remaining < 0 || reserve_available < 0 {
            return Err(Error::InvalidAmount);
        }
        if reserve_remaining == 0 {
            storage::set_reserve_reclaimed(env, true);
            let event = ReserveReclaimed {
                destination: destination.clone(),
                amount: 0,
                sweep_id,
                fully_reclaimed: true,
                remaining_reserve: 0,
            };
            Self::emit_and_store_reserve_event(env, event)?;
            return Ok(0);
        }
        let reclaim_amount = if reserve_available < reserve_remaining {
            reserve_available
        } else {
            reserve_remaining
        };
        let new_available = reserve_available
            .checked_sub(reclaim_amount)
            .ok_or(Error::InvalidAmount)?;
        let new_remaining = reserve_remaining
            .checked_sub(reclaim_amount)
            .ok_or(Error::InvalidAmount)?;
        storage::set_available_reserve(env, new_available);
        storage::set_base_reserve_remaining(env, new_remaining);
        storage::set_reserve_reclaimed(env, new_remaining == 0);
        let event = ReserveReclaimed {
            destination: destination.clone(),
            amount: reclaim_amount,
            sweep_id,
            fully_reclaimed: new_remaining == 0,
            remaining_reserve: new_remaining,
        };
        Self::emit_and_store_reserve_event(env, event)?;
        Ok(reclaim_amount)
    }

    fn emit_and_store_reserve_event(env: &Env, event: ReserveReclaimed) -> Result<(), Error> {
        events::emit_reserve_reclaimed(
            env,
            event.destination.clone(),
            event.amount,
            event.sweep_id,
            event.fully_reclaimed,
            event.remaining_reserve,
        );
        let event_count = storage::get_reserve_event_count(env);
        let next_count = event_count.checked_add(1).ok_or(Error::InvalidAmount)?;
        storage::set_last_reserve_event(env, &event);
        storage::set_reserve_event_count(env, next_count);
        Ok(())
    }
}
