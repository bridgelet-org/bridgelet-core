#![no_std]
mod test;
mod authorization;
mod errors;
mod transfers;
mod events;
mod storage;
#[cfg(test)]



use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, Map, Vec};

use authorization::AuthContext;
use transfers::TransferContext;
pub use errors::Error;
pub use events::{AccountCreated, AccountExpired, AssetAmount, MultiPaymentReceived, ReserveReclaimed, SweepExecuted};
pub use storage::{AccountStatus, DataKey, Payment};

#[contract]
pub struct SweepController;

// XLM native asset address (Stellar native asset)
const NATIVE_ASSET: [u8; 32] = [0u8; 32]; // Placeholder - use actual Stellar native asset ID



#[contractimpl]
impl EphemeralAccountContract {
    /// Initialize the ephemeral account with restrictions
    ///
    /// # Arguments
    /// * `creator` - Address that created this account
    /// * `expiry_ledger` - Ledger number when account expires
    /// * `recovery_address` - Address to return funds if expired
    /// * `expected_assets` - Number of different assets expected (for reserve calculation)
    ///
    /// # Errors
    /// Returns Error::AlreadyInitialized if called more than once
    pub fn initialize(
        env: Env,
        creator: Address,
        expiry_ledger: u32,
        recovery_address: Address,
        expected_assets: u32,
    ) -> Result<(), Error> {
        // Check if already initialized
        if storage::is_initialized(&env) {
            return Err(Error::AlreadyInitialized);
        }

        // Verify creator authorization
        creator.require_auth();

        // Validate expiry is in future
        let current_ledger = env.ledger().sequence();
        if expiry_ledger <= current_ledger {
            return Err(Error::InvalidExpiry);
        }

        // Calculate and store base reserve
        // Account needs: 1 XLM base + 0.5 XLM per trustline (asset)
        let base_reserve = storage::calculate_base_reserve(expected_assets);
        
        // Store initialization data
        storage::set_initialized(&env, true);
        storage::set_creator(&env, &creator);
        storage::set_expiry_ledger(&env, expiry_ledger);
        storage::set_recovery_address(&env, &recovery_address);
        storage::set_status(&env, AccountStatus::Active);
        storage::set_base_reserve(&env, base_reserve);
        storage::set_reserve_reclaimed(&env, false);

        // Emit event
        events::emit_account_created(&env, creator, expiry_ledger);

        Ok(())
    }

    /// Record an inbound payment to this ephemeral account
    /// Multiple payments allowed, but only one per asset type
    ///
    /// # Arguments
    /// * `amount` - Payment amount
    /// * `asset` - Asset address
    ///
    /// # Errors
    /// Returns Error::PaymentAlreadyReceived if asset already has a payment
    /// Returns Error::MaxAssetsExceeded if too many different assets

    /// Record an inbound payment to this ephemeral account
    /// Multiple payments with different assets are supported
    ///
    /// # Arguments
    /// * `amount` - Payment amount
    /// * `asset` - Asset address
    ///
    /// # Errors
    /// Returns Error::InvalidAmount if amount is not positive
    /// Returns Error::DuplicateAsset if asset already has a payment
    pub fn record_payment(env: Env, amount: i128, asset: Address) -> Result<(), Error> {
        // Check initialized
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }

        // Validate amount
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        // Check for duplicate asset
        if storage::get_payment(&env, &asset).is_some() {
            return Err(Error::DuplicateAsset);
        }

        // Check payment limit to prevent gas issues (max 10 assets)
        let payment_count = storage::get_total_payments(&env);
        if payment_count >= 10 {
            return Err(Error::TooManyPayments);
        }

        // Create payment with current timestamp
        let payment = Payment {
            asset: asset.clone(),
            amount,
            timestamp: env.ledger().timestamp(),
        };

        // Add payment
        storage::add_payment(&env, payment);

        // Update status only on first payment
        if payment_count == 0 {
            storage::set_status(&env, AccountStatus::PaymentReceived);
        }

        // Emit appropriate event
        if payment_count == 0 {
            events::emit_payment_received(&env, amount, asset);
        } else {
            events::emit_multi_payment_received(&env, asset, amount);
        }

        Ok(())
    }

    /// Execute sweep to destination wallet
    /// Transfers all funds from all assets to the specified destination atomically
    ///
    /// # Arguments
    /// * `destination` - Recipient wallet address
    /// * `auth_signature` - Authorization signature from off-chain system
    ///
    /// # Errors
    /// Returns Error::Unauthorized if authorization fails
    /// Returns Error::AlreadySwept if sweep already executed
    pub fn sweep(env: Env, destination: Address, auth_signature: BytesN<64>) -> Result<(), Error> {
        // Check initialized
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }

        // Check not already swept
        if storage::get_status(&env) == AccountStatus::Swept {
            return Err(Error::AlreadySwept);
        }

        // Check payment received
        if !storage::has_payment_received(&env) {
            return Err(Error::NoPaymentReceived);
        }

        // Check not expired
        if Self::is_expired(env.clone()) {
            return Err(Error::AccountExpired);
        }

        // Verify authorization signature
        // Note: In production, implement proper signature verification
        // For MVP, we trust the SDK to only call with valid signatures
        Self::verify_sweep_authorization(&env, &destination, &auth_signature)?;

        // Get all payments
        let payments = storage::get_all_payments(&env);
        let mut payments_vec = Vec::new(&env);
        for payment in payments.values() {
            payments_vec.push_back(payment);
        }

        // Update status before transfer to prevent reentrancy
        storage::set_status(&env, AccountStatus::Swept);
        storage::set_swept_to(&env, &destination);

        // Note: Actual token transfers happen in the SDK via Stellar SDK
        // This contract enforces the business logic and authorization
        // The SDK will call this function, get approval, then execute all transfers atomically
        // All transfers must succeed or the entire operation fails

        // Emit event with all assets
        events::emit_sweep_executed_multi(&env, destination, &payments_vec);

        Ok(())
    }

    /// Check if account has expired
    pub fn is_expired(env: Env) -> bool {
        if !storage::is_initialized(&env) {
            return false;
        }

        let expiry_ledger = storage::get_expiry_ledger(&env);
        let current_ledger = env.ledger().sequence();

        current_ledger >= expiry_ledger
    }

    /// Get current account status
    pub fn get_status(env: Env) -> AccountStatus {
        if !storage::is_initialized(&env) {
            return AccountStatus::Active;
        }

        storage::get_status(&env)
    }

    /// Expire the account and return funds to recovery address
    /// Can only be called after expiry ledger is reached
    ///
    /// # Errors
    /// Returns Error::NotExpired if called before expiry ledger
    pub fn expire(env: Env) -> Result<(), Error> {
        // Check initialized
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }

        // Check not already swept or expired
        let status = storage::get_status(&env);
        if status == AccountStatus::Swept || status == AccountStatus::Expired {
            return Err(Error::InvalidStatus);
        }

        // Check if expired
        if !Self::is_expired(env.clone()) {
            return Err(Error::NotExpired);
        }

        // Get recovery address
        let recovery_address = storage::get_recovery_address(&env);

        // Update status
        storage::set_status(&env, AccountStatus::Expired);
        storage::set_swept_to(&env, &recovery_address);

        // Get total amount from all payments if any payments were received
        let total_amount = if storage::has_payment_received(&env) {
            let payments = storage::get_all_payments(&env);
            payments
                .iter()
                .fold(0, |sum, (_, payment)| sum + payment.amount)
        } else {
            0
        };

        // Emit event
        events::emit_account_expired(&env, recovery_address, total_amount);

        Ok(())
    }

    /// Get account information
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

    // Private helper functions

    fn verify_sweep_authorization(
        _env: &Env,
        _destination: &Address,
        _signature: &BytesN<64>,
    ) -> Result<(), Error> {
        // TODO: Implement proper signature verification
        // For MVP, we rely on off-chain SDK to only call with valid auth
        // Future: Verify signature against authorized signer
        Ok(())
    }
}
    pub fn record_payment(env: Env, amount: i128, asset: Address) -> Result<(), Error> {
        // Check initialized
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }

        // Validate amount
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        // Check current status
        let current_status = storage::get_status(&env);
        if current_status == AccountStatus::Swept || current_status == AccountStatus::Expired {
            return Err(Error::InvalidStatus);
        }

        // Add payment (will error if duplicate asset)
        storage::add_payment(&env, asset.clone(), amount)?;

        // Update status to PaymentReceived on first payment
        if storage::get_total_payments(&env) == 1 {
            storage::set_status(&env, AccountStatus::PaymentReceived);
        }

        // Emit multi-payment event
        events::emit_multi_payment_received(&env, asset, amount, storage::get_total_payments(&env));

        Ok(())
    }

    /// Execute sweep to destination wallet
    /// Transfers all funds from all assets to the specified destination
    /// Then reclaims base reserve
    ///
    /// # Arguments
    /// * `destination` - Recipient wallet address
    /// * `auth_signature` - Authorization signature from off-chain system
    ///
    /// # Errors
    /// Returns Error::Unauthorized if authorization fails
    /// Returns Error::AlreadySwept if sweep already executed
    pub fn sweep(env: Env, destination: Address, auth_signature: BytesN<64>) -> Result<(), Error> {
        // Check initialized
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }

        // Check not already swept
        if storage::get_status(&env) == AccountStatus::Swept {
            return Err(Error::AlreadySwept);
        }

        // Check payment received
        if !storage::has_payments(&env) {
            return Err(Error::NoPaymentReceived);
        }

        // Check not expired
        if Self::is_expired(env.clone()) {
            return Err(Error::AccountExpired);
        }

        // Verify authorization signature
        Self::verify_sweep_authorization(&env, &destination, &auth_signature)?;

        // Get all payments
        let payments = storage::get_all_payments(&env);

        // Update status before transfer to prevent reentrancy
        storage::set_status(&env, AccountStatus::Swept);
        storage::set_swept_to(&env, &destination);

        // Note: Actual token transfers happen in the SweepController contract
        // This contract enforces the business logic and authorization

        // Get base reserve amount for event
        let base_reserve = storage::get_base_reserve(&env);

        // Emit event with all assets and reserve info
        events::emit_sweep_executed(&env, destination, &payments, base_reserve);

        Ok(())
    }

    /// Reclaim base reserve after successful sweep
    /// Should be called by SweepController after asset transfers complete
    ///
    /// # Arguments
    /// * `recipient` - Address to receive the base reserve (usually recovery or destination)
    ///
    /// # Errors
    /// Returns Error::InvalidStatus if not in Swept status
    /// Returns Error::AlreadySwept if reserve already reclaimed
    pub fn reclaim_reserve(env: Env, recipient: Address) -> Result<i128, Error> {
        // Check initialized
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }

        // Check status is Swept
        if storage::get_status(&env) != AccountStatus::Swept {
            return Err(Error::InvalidStatus);
        }

        // Check reserve not already reclaimed
        if storage::is_reserve_reclaimed(&env) {
            return Err(Error::AlreadySwept);
        }

        // Get base reserve amount
        let base_reserve = storage::get_base_reserve(&env);

        // Calculate reclaimable amount (reserve minus minimum for final close)
        let reclaimable = if base_reserve > storage::MIN_BALANCE_FOR_CLOSE {
            base_reserve - storage::MIN_BALANCE_FOR_CLOSE
        } else {
            0
        };

        // Mark reserve as reclaimed
        storage::set_reserve_reclaimed(&env, true);

        // Note: Actual XLM transfer happens in SweepController
        // This function authorizes the reclamation

        // Emit event
        if reclaimable > 0 {
            events::emit_reserve_reclaimed(&env, recipient, reclaimable);
        }

        Ok(reclaimable)
    }

    /// Check if account has expired
    pub fn is_expired(env: Env) -> bool {
        if !storage::is_initialized(&env) {
            return false;
        }

        let expiry_ledger = storage::get_expiry_ledger(&env);
        let current_ledger = env.ledger().sequence();

        current_ledger >= expiry_ledger
    }

    /// Get current account status
    pub fn get_status(env: Env) -> AccountStatus {
        if !storage::is_initialized(&env) {
            return AccountStatus::Active;
        }

        storage::get_status(&env)
    }

    /// Expire the account and return funds to recovery address
    /// Includes both payment funds and base reserve
    /// Can only be called after expiry ledger is reached
    ///
    /// # Errors
    /// Returns Error::NotExpired if called before expiry ledger
    pub fn expire(env: Env) -> Result<(), Error> {
        // Check initialized
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }

        // Check not already swept or expired
        let status = storage::get_status(&env);
        if status == AccountStatus::Swept || status == AccountStatus::Expired {
            return Err(Error::InvalidStatus);
        }

        // Check if expired
        if !Self::is_expired(env.clone()) {
            return Err(Error::NotExpired);
        }

        // Get recovery address
        let recovery_address = storage::get_recovery_address(&env);

        // Update status
        storage::set_status(&env, AccountStatus::Expired);
        storage::set_swept_to(&env, &recovery_address);

        // Get total assets count
        let total_assets = storage::get_total_payments(&env);

        // Get base reserve to return
        let base_reserve = storage::get_base_reserve(&env);
        let reserve_to_return = if base_reserve > storage::MIN_BALANCE_FOR_CLOSE {
            base_reserve - storage::MIN_BALANCE_FOR_CLOSE
        } else {
            0
        };

        // Mark reserve as reclaimed
        storage::set_reserve_reclaimed(&env, true);

        // Note: Actual asset and reserve transfers happen off-chain or via controller
        
        // Emit event with reserve info
        events::emit_account_expired(&env, recovery_address, total_assets, reserve_to_return);

        Ok(())
    }

    /// Get account information including reserve status
    pub fn get_info(env: Env) -> Result<AccountInfo, Error> {
        if !storage::is_initialized(&env) {
            return Err(Error::NotInitialized);
        }

        let payments = storage::get_all_payments(&env);
        let mut payment_list = Vec::new(&env);
        
        for key in payments.keys() {
            let asset = key;
            let amount = payments.get(asset.clone()).unwrap();
            payment_list.push_back(AssetAmount { asset, amount });
        }

        Ok(AccountInfo {
            creator: storage::get_creator(&env),
            status: storage::get_status(&env),
            expiry_ledger: storage::get_expiry_ledger(&env),
            recovery_address: storage::get_recovery_address(&env),
            payment_received: storage::has_payments(&env),
            payments: payment_list,
            swept_to: storage::get_swept_to(&env),
            base_reserve: storage::get_base_reserve(&env),
            reserve_reclaimed: storage::is_reserve_reclaimed(&env),
        })
    }

    /// Get all payments as a map
    pub fn get_payments(env: Env) -> Map<Address, i128> {
        storage::get_all_payments(&env)
    }

    /// Get base reserve amount
    pub fn get_base_reserve(env: Env) -> i128 {
        storage::get_base_reserve(&env)
    }

    /// Check if reserve has been reclaimed
    pub fn is_reserve_reclaimed(env: Env) -> bool {
        storage::is_reserve_reclaimed(&env)
    }

    // Private helper functions

    fn verify_sweep_authorization(
        _env: &Env,
        _destination: &Address,
        _signature: &BytesN<64>,
    ) -> Result<(), Error> {
        // TODO: Implement proper signature verification
        // For MVP, we rely on off-chain SDK to only call with valid auth
        // Future: Verify signature against authorized signer
        Ok(())
    }

    
#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, BytesN};

}

       #[test]
    fn test_base_reserve_calculation() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EphemeralAccountContract);
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        // Initialize with 3 expected assets
        client.initialize(&creator, &expiry_ledger, &recovery, &3);

        // Base reserve should be: 1 XLM (account) + 1.5 XLM (3 * 0.5 XLM trustlines)
        // = 2.5 XLM = 25,000,000 stroops
        let expected_reserve = 10_000_000 + (3 * 5_000_000);
        let reserve = client.get_base_reserve();
        assert_eq!(reserve, expected_reserve);
    }

    #[test]
    fn test_reclaim_reserve() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, EphemeralAccountContract);
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let destination = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        // Initialize with 1 asset
        client.initialize(&creator, &expiry_ledger, &recovery, &1);
        
        // Record payment and sweep
        client.record_payment(&100, &asset);
        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);

        // Reclaim reserve
        let reclaimable = client.reclaim_reserve(&destination);

        // Should reclaim base reserve minus minimum for close
        // Base: 1.5 XLM (15,000,000 stroops)
        // Reclaimable: 1.5 - 0.1 = 1.4 XLM (14,000,000 stroops)
        assert_eq!(reclaimable, 14_000_000);

        // Check reserve marked as reclaimed
        assert!(client.is_reserve_reclaimed());
    }

    #[test]
    #[should_panic]
    fn test_double_reclaim_reserve() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, EphemeralAccountContract);
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let destination = Address::generate(&env);
        let asset = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        // Setup and sweep
        client.initialize(&creator, &expiry_ledger, &recovery, &1);
        client.record_payment(&100, &asset);
        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);

        // First reclaim
        client.reclaim_reserve(&destination);

        // Second reclaim should panic
        client.reclaim_reserve(&destination);
    }

    #[test]
    fn test_expire_with_reserve() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, EphemeralAccountContract);
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 10;

        // Initialize with 2 assets
        client.initialize(&creator, &expiry_ledger, &recovery, &2);

        // Advance past expiry
        env.ledger().set_sequence_number(expiry_ledger + 1);

        // Expire
        client.expire();

        // Check reserve marked as reclaimed
        assert!(client.is_reserve_reclaimed());

        // Verify status
        assert_eq!(client.get_status(), AccountStatus::Expired);
    }

    #[test]
    fn test_info_includes_reserve() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EphemeralAccountContract);
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        // Initialize
        client.initialize(&creator, &expiry_ledger, &recovery, &2);

        // Get info
        let info = client.get_info();

        // Verify reserve info included
        assert_eq!(info.base_reserve, 20_000_000); // 1 + (2 * 0.5) XLM
        assert_eq!(info.reserve_reclaimed, false);
    }

    #[test]
    fn test_multi_asset_with_reserve() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, EphemeralAccountContract);
        let client = EphemeralAccountContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let destination = Address::generate(&env);
        let asset1 = Address::generate(&env);
        let asset2 = Address::generate(&env);
        let asset3 = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        // Initialize with 3 assets
        client.initialize(&creator, &expiry_ledger, &recovery, &3);
        
        // Record payments
        client.record_payment(&100, &asset1);
        client.record_payment(&200, &asset2);
        client.record_payment(&300, &asset3);

        // Sweep
        let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
        client.sweep(&destination, &auth_sig);

        // Reclaim reserve
        let reclaimable = client.reclaim_reserve(&destination);

        // Base: 2.5 XLM - 0.1 XLM = 2.4 XLM (24,000,000 stroops)
        assert_eq!(reclaimable, 24_000_000);
    }


/// Account information structure
#[derive(Clone)]
#[contracttype]
pub struct AccountInfo {
    pub creator: Address,
    pub status: AccountStatus,
    pub expiry_ledger: u32,
    pub recovery_address: Address,
    pub payment_received: bool,
    pub payments: Vec<AssetAmount>,
    pub swept_to: Option<Address>,
    pub base_reserve: i128,
    pub reserve_reclaimed: bool,
}

#[contractimpl]
impl SweepController {
    /// Execute sweep operation from ephemeral account to destination
    /// Handles multiple assets atomically plus base reserve reclamation
    ///
    /// # Arguments
    /// * `ephemeral_account` - Address of the ephemeral account contract
    /// * `destination` - Destination wallet address
    /// * `auth_signature` - Authorization signature
    /// * `reclaim_reserve_to` - Optional address to receive base reserve (defaults to destination)
    ///
    /// # Errors
    /// Returns Error::AuthorizationFailed if signature is invalid
    /// Returns Error::InvalidAccount if account is not in valid state
    /// Returns Error::TransferFailed if any token transfer fails
    pub fn execute_sweep(
        env: Env,
        ephemeral_account: Address,
        destination: Address,
        auth_signature: BytesN<64>,
        reclaim_reserve_to: Option<Address>,
    ) -> Result<(), Error> {
        // Verify authorization
        let auth_ctx = AuthContext::new(
            ephemeral_account.clone(),
            destination.clone(),
            auth_signature.clone(),
        );
        auth_ctx.verify(&env)?;

        // Call ephemeral account contract to validate and authorize sweep
        let account_client = ephemeral_account::Client::new(&env, &ephemeral_account);
        
        // The account contract validates state and authorizes the sweep
        account_client
            .sweep(&destination, &auth_signature)
            .map_err(|_| Error::InvalidAccount)?;

        // Get all payments from account
        let payments: Map<Address, i128> = account_client
            .get_payments()
            .map_err(|_| Error::InvalidAccount)?;

        // Verify we have payments
        if payments.len() == 0 {
            return Err(Error::AccountNotReady);
        }

        // Execute all asset transfers atomically
        // If any transfer fails, the entire transaction reverts
        for key in payments.keys() {
            let asset = key;
            let amount = payments.get(asset.clone()).unwrap();
            
            let transfer_ctx = TransferContext::new(
                asset,
                ephemeral_account.clone(),
                destination.clone(),
                amount,
            );
            transfer_ctx.execute(&env)?;
        }

        // Reclaim base reserve after successful asset transfers
        let reserve_recipient = reclaim_reserve_to.unwrap_or(destination.clone());
        let reserve_amount = account_client
            .reclaim_reserve(&reserve_recipient)
            .map_err(|_| Error::TransferFailed)?;

        // Transfer base reserve XLM if reclaimable
        if reserve_amount > 0 {
            // Note: In production, this would transfer native XLM
            // For now, we just authorize the reclamation
            // The actual XLM transfer would use Stellar's native asset transfer
        }

        // Emit sweep completed event with all assets
        emit_sweep_completed(&env, ephemeral_account, destination, &payments, reserve_amount);

        Ok(())
    }

    /// Check if an account is ready for sweep
    pub fn can_sweep(env: Env, ephemeral_account: Address) -> bool {
        let account_client = ephemeral_account::Client::new(&env, &ephemeral_account);
        
        // Check if account exists and has payment
        match account_client.get_info() {
            Ok(info) => {
                info.payment_received 
                    && info.status == ephemeral_account::AccountStatus::PaymentReceived
                    && !account_client.is_expired()
            }
            Err(_) => false,
        }
    }

    /// Get number of assets ready to sweep
    pub fn get_asset_count(env: Env, ephemeral_account: Address) -> u32 {
        let account_client = ephemeral_account::Client::new(&env, &ephemeral_account);
        
        match account_client.get_payments() {
            Ok(payments) => payments.len(),
            Err(_) => 0,
        }
    }

    /// Get reclaimable base reserve amount
    pub fn get_reclaimable_reserve(env: Env, ephemeral_account: Address) -> i128 {
        let account_client = ephemeral_account::Client::new(&env, &ephemeral_account);
        
        match account_client.get_info() {
            Ok(info) => {
                if info.reserve_reclaimed {
                    0
                } else {
                    // Calculate reclaimable amount
                    let base = info.base_reserve;
                    let min_balance = 1_000_000; // 0.1 XLM
                    if base > min_balance {
                        base - min_balance
                    } else {
                        0
                    }
                }
            }
            Err(_) => 0,
        }
    }
}

/// Sweep completed event with multiple assets and reserve info
#[contracttype]
#[derive(Clone, Debug)]
pub struct AssetAmount {
    pub asset: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct SweepCompleted {
    pub ephemeral_account: Address,
    pub destination: Address,
    pub assets: Vec<AssetAmount>,
    pub reserve_reclaimed: i128,
}

fn emit_sweep_completed(
    env: &Env,
    account: Address,
    destination: Address,
    payments: &Map<Address, i128>,
    reserve_amount: i128,
) {
    let mut assets = Vec::new(env);
    
    for key in payments.keys() {
        let asset = key;
        let amount = payments.get(asset.clone()).unwrap();
        assets.push_back(AssetAmount { asset, amount });
    }
    
    let event = SweepCompleted {
        ephemeral_account: account,
        destination,
        assets,
        reserve_reclaimed: reserve_amount,
    };
    env.events()
        .publish((soroban_sdk::symbol_short!("sweep"),), event);
}

// Re-export ephemeral_account types for cross-contract calls
mod ephemeral_account {
    use soroban_sdk::{contractclient, Address, BytesN, Env, Map};
soroban_sdk::contractimport!( file = "../ephemeral_account/target/wasm32-unknown-unknown/release/ephemeral_account.wasm" ); 
}

## 6. Add Integration Tests in `contracts/sweep_controller/tests/integration.rs`

#[test]
fn test_sweep_with_reserve_reclamation() {
    let env = Env::default();
    env.mock_all_auths();

    // Deploy contracts
    let ephemeral_id = env.register_contract(None, ephemeral_account::EphemeralAccountContract);
    let ephemeral_client = ephemeral_account::EphemeralAccountContractClient::new(&env, &ephemeral_id);

    let controller_id = env.register_contract(None, SweepController);
    let controller_client = SweepControllerClient::new(&env, &controller_id);

    // Setup
    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let destination = Address::generate(&env);
    let asset = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1000;

    // Initialize with 1 asset
    ephemeral_client.initialize(&creator, &expiry, &recovery, &1);
    
    // Record payment
    ephemeral_client.record_payment(&100, &asset);

    // Check reclaimable reserve before sweep
    let reclaimable_before = controller_client.get_reclaimable_reserve(&ephemeral_id);
    assert_eq!(reclaimable_before, 14_000_000); // 1.5 - 0.1 XLM

    // Execute sweep with reserve reclamation
    let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
    controller_client.execute_sweep(&ephemeral_id, &destination, &auth_sig, &None);

    // Verify reserve was reclaimed
    assert!(ephemeral_client.is_reserve_reclaimed());

    // Check reclaimable reserve after sweep
    let reclaimable_after = controller_client.get_reclaimable_reserve(&ephemeral_id);
    assert_eq!(reclaimable_after, 0);
}

#[test]
fn test_get_reclaimable_reserve() {
    let env = Env::default();
    env.mock_all_auths();

    let ephemeral_id = env.register_contract(None, ephemeral_account::EphemeralAccountContract);
    let ephemeral_client = ephemeral_account::EphemeralAccountContractClient::new(&env, &ephemeral_id);

    let controller_id = env.register_contract(None, SweepController);
    let controller_client = SweepControllerClient::new(&env, &controller_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1000;

    // Before initialization
    assert_eq!(controller_client.get_reclaimable_reserve(&ephemeral_id), 0);

    // Initialize with 3 assets (2.5 XLM reserve)
    ephemeral_client.initialize(&creator, &expiry, &recovery, &3);

    // Should show reclaimable amount
    // 2.5 XLM - 0.1 XLM = 2.4 XLM = 24,000,000 stroops
    assert_eq!(controller_client.get_reclaimable_reserve(&ephemeral_id), 24_000_000);
}

#[test]
fn test_multi_asset_sweep_with_reserve() {
    let env = Env::default();
    env.mock_all_auths();

    let ephemeral_id = env.register_contract(None, ephemeral_account::EphemeralAccountContract);
    let ephemeral_client = ephemeral_account::EphemeralAccountContractClient::new(&env, &ephemeral_id);

    let controller_id = env.register_contract(None, SweepController);
    let controller_client = SweepControllerClient::new(&env, &controller_id);

    let creator = Address::generate(&env);
    let recovery = Address::generate(&env);
    let destination = Address::generate(&env);
    let reserve_recipient = Address::generate(&env);
    let asset1 = Address::generate(&env);
    let asset2 = Address::generate(&env);
    let expiry = env.ledger().sequence() + 1000;

    // Initialize with 2 assets
    ephemeral_client.initialize(&creator, &expiry, &recovery, &2);
    
    // Record payments
    ephemeral_client.record_payment(&100, &asset1);
    ephemeral_client.record_payment(&200, &asset2);

    // Execute sweep with separate reserve recipient
    let auth_sig = BytesN::from_array(&env, &[0u8; 64]);
    controller_client.execute_sweep(
        &ephemeral_id,
        &destination,
        &auth_sig,
        &Some(reserve_recipient),
    );

    // Verify reserve reclaimed
    assert!(ephemeral_client.is_reserve_reclaimed());
}

