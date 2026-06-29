//! Error types for event backfilling operations.

use thiserror::Error;

/// Errors that can occur during event backfilling.
#[derive(Error, Debug)]
pub enum BackfillError {
    /// Error from the RPC client
    #[error("RPC error: {0}")]
    RpcError(String),

    /// HTTP request error
    #[error("HTTP request error: {0}")]
    HttpError(#[from] reqwest::Error),

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// No events found
    #[error("No events found for the given criteria")]
    NoEventsFound,

    /// Pagination error
    #[error("Pagination error: {0}")]
    PaginationError(String),

    /// XDR decoding error
    #[error("XDR decoding error: {0}")]
    XdrError(String),
}

/// Result type for event backfilling operations.
pub type Result<T> = std::result::Result<T, BackfillError>;
