use soroban_sdk::{contracttype, Address, Bytes, Vec};

// Represents a payment received by the ephemeral account.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Payment {
    pub asset: Address,
    pub amount: i128,
    pub timestamp: u64,
}
// The current status of an ephemeral account.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
#[repr(u32)]
pub enum AccountStatus {
    Active = 0,
    PaymentReceived = 1,
    Swept = 2,
    Expired = 3,
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
    pub payment_count: u32,
    pub payments: Vec<Payment>,
    pub swept_to: Option<Address>,
}

/// Request to initialize a single ephemeral account
#[contracttype]
#[derive(Clone, Debug)]
pub struct AccountInitRequest {
    pub expiry_ledger: u32,
    pub recovery_address: Address,
}

/// Result of initializing an ephemeral account
#[contracttype]
#[derive(Clone, Debug)]
pub struct AccountInitResult {
    pub account_address: Address,
    pub success: bool,
    pub error: Option<Bytes>,
}

/// Pagination cursor for list-returning functions.
/// Opaque to callers; encode/decode via `to_xdr`/`from_xdr` or base64.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginationCursor {
    /// Index of the next item to return (0-based).
    pub next_index: u32,
    /// Total number of items available (for UI progress).
    pub total_count: u32,
}

/// Sentinel value for "no next cursor" - u32::MAX.
pub const NO_CURSOR: u32 = u32::MAX;

/// Paginated response for Payment items.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginatedPaymentResponse {
    /// Items in this page.
    pub items: Vec<Payment>,
    /// Cursor for the next page (next_index), or NO_CURSOR if no more pages.
    pub next_cursor_index: u32,
    /// Total count of all items (for first page).
    pub total_count: u32,
}

/// Paginated response for AccountInitResult items.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginatedAccountInitResultResponse {
    /// Items in this page.
    pub items: Vec<AccountInitResult>,
    /// Cursor for the next page (next_index), or NO_CURSOR if no more pages.
    pub next_cursor_index: u32,
    /// Total count of all items (for first page).
    pub total_count: u32,
}

/// Standard pagination parameters for list-returning functions.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginationParams {
    /// Maximum items per page (1-1000, default 50).
    pub limit: u32,
    /// Opaque cursor from previous page (next_index), or NO_CURSOR for first page.
    pub cursor_index: u32,
}
