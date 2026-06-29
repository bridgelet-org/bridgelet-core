//! Types for event backfilling configuration and data structures.

use serde::{Deserialize, Serialize};

/// Configuration for event backfilling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillConfig {
    /// RPC server URL (e.g., "https://soroban-testnet.stellar.org")
    pub rpc_url: String,
    
    /// Contract ID to filter events for
    pub contract_id: String,
    
    /// Starting ledger sequence (inclusive)
    pub start_ledger: u32,
    
    /// Optional ending ledger sequence (exclusive). If None, fetches to latest.
    pub end_ledger: Option<u32>,
    
    /// Number of events to fetch per batch (max 10000 per RPC limits)
    pub batch_size: u32,
}

impl BackfillConfig {
    /// Creates a new BackfillConfig with sensible defaults.
    pub fn new(rpc_url: String, contract_id: String, start_ledger: u32) -> Self {
        Self {
            rpc_url,
            contract_id,
            start_ledger,
            end_ledger: None,
            batch_size: 100,
        }
    }

    /// Validates the configuration.
    pub fn validate(&self) -> crate::Result<()> {
        if self.rpc_url.is_empty() {
            return Err(crate::BackfillError::InvalidConfig(
                "RPC URL cannot be empty".to_string(),
            ));
        }
        if self.contract_id.is_empty() {
            return Err(crate::BackfillError::InvalidConfig(
                "Contract ID cannot be empty".to_string(),
            ));
        }
        if self.batch_size == 0 || self.batch_size > 10000 {
            return Err(crate::BackfillError::InvalidConfig(
                "Batch size must be between 1 and 10000".to_string(),
            ));
        }
        if let Some(end) = self.end_ledger {
            if end <= self.start_ledger {
                return Err(crate::BackfillError::InvalidConfig(
                    "End ledger must be greater than start ledger".to_string(),
                ));
            }
        }
        Ok(())
    }
}

/// Filter for contract events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Event type filter (contract, system, diagnostic)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    
    /// Contract IDs to filter (max 5)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contract_ids: Vec<String>,
    
    /// Topic filters
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub topics: Vec<Vec<String>>,
}

impl EventFilter {
    /// Creates a new filter for a specific contract.
    pub fn for_contract(contract_id: String) -> Self {
        Self {
            r#type: Some("contract".to_string()),
            contract_ids: vec![contract_id],
            topics: vec![],
        }
    }

    /// Adds a topic filter.
    pub fn with_topic(mut self, topic: Vec<String>) -> Self {
        self.topics.push(topic);
        self
    }
}

/// A parsed contract event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEvent {
    /// Event ID
    pub id: String,
    
    /// Ledger sequence where the event occurred
    pub ledger: u32,
    
    /// Timestamp when the ledger closed
    pub ledger_closed_at: String,
    
    /// Contract ID that emitted the event
    pub contract_id: String,
    
    /// Event type (contract, system, diagnostic)
    pub event_type: String,
    
    /// Event topics (base64-encoded XDR)
    pub topics: Vec<String>,
    
    /// Event data (base64-encoded XDR)
    pub data: String,
    
    /// Whether the event was in a successful contract call
    pub in_successful_contract_call: bool,
    
    /// Paging token for pagination
    pub paging_token: String,
}

impl ContractEvent {
    /// Returns the event data as base64-encoded bytes.
    pub fn data_as_bytes(&self) -> crate::Result<Vec<u8>> {
        use base64::{Engine as _, engine::general_purpose};
        general_purpose::STANDARD.decode(&self.data)
            .map_err(|e| crate::BackfillError::XdrError(format!("Base64 decode error: {}", e)))
    }

    /// Returns a topic as base64-encoded bytes.
    pub fn topic_as_bytes(&self, index: usize) -> crate::Result<Vec<u8>> {
        if index >= self.topics.len() {
            return Err(crate::BackfillError::XdrError(
                "Topic index out of bounds".to_string(),
            ));
        }
        
        use base64::{Engine as _, engine::general_purpose};
        general_purpose::STANDARD.decode(&self.topics[index])
            .map_err(|e| crate::BackfillError::XdrError(format!("Base64 decode error: {}", e)))
    }
}

/// RPC request for getEvents.
#[derive(Debug, Serialize)]
pub(crate) struct GetEventsRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: GetEventsParams,
}

/// Parameters for getEvents RPC call.
#[derive(Debug, Serialize)]
pub(crate) struct GetEventsParams {
    pub start_ledger: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_ledger: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<EventFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xdr_format: Option<String>,
}

/// Pagination parameters.
#[derive(Debug, Serialize)]
pub(crate) struct Pagination {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    pub limit: u32,
}

/// RPC response from getEvents.
#[derive(Debug, Deserialize)]
pub(crate) struct GetEventsResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: GetEventsResult,
}

/// Result field from getEvents response.
#[derive(Debug, Deserialize)]
pub(crate) struct GetEventsResult {
    pub events: Vec<RpcEvent>,
    pub latest_ledger: String,
}

/// Event as returned by RPC.
#[derive(Debug, Deserialize)]
pub(crate) struct RpcEvent {
    pub id: String,
    pub ledger: String,
    pub ledger_closed_at: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub contract_id: Option<String>,
    #[serde(default)]
    pub topic: Vec<String>,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub in_successful_contract_call: bool,
    #[serde(default)]
    pub paging_token: String,
}
