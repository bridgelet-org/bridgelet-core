//! Event backfiller implementation for querying historical Soroban contract events.

use crate::{
    error::{BackfillError, Result},
    types::{BackfillConfig, ContractEvent, EventFilter, GetEventsRequest, GetEventsParams,
            GetEventsResponse, Pagination, RpcEvent},
};
use reqwest::Client;
use std::time::Duration;

/// Event backfiller for querying historical contract events from Stellar RPC.
pub struct EventBackfiller {
    config: BackfillConfig,
    client: Client,
}

impl EventBackfiller {
    /// Creates a new EventBackfiller with the given configuration.
    pub fn new(config: BackfillConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Backfills all events for the configured contract within the ledger range.
    ///
    /// This method fetches events in batches, handling pagination automatically.
    /// Returns a vector of all events found, ordered by ledger sequence.
    pub async fn backfill_events(&self) -> Result<Vec<ContractEvent>> {
        self.config.validate()?;

        let mut all_events = Vec::new();
        let mut cursor: Option<String> = None;
        let mut current_ledger = self.config.start_ledger;

        loop {
            let request = self.build_request(current_ledger, cursor.clone())?;
            let response = self.send_request(request).await?;

            if response.result.events.is_empty() {
                break;
            }

            // Get the cursor and last ledger before consuming events
            let last_event_ledger = response.result.events.last().and_then(|e| e.ledger.parse().ok());
            cursor = response.result.events.last().map(|e| e.paging_token.clone());

            let events: Vec<ContractEvent> = response
                .result
                .events
                .into_iter()
                .map(|e| self.convert_rpc_event(e))
                .collect();

            all_events.extend(events.clone());

            // Check if we've reached the end ledger
            if let Some(end_ledger) = self.config.end_ledger {
                let max_ledger = all_events
                    .iter()
                    .map(|e| e.ledger)
                    .max()
                    .unwrap_or(current_ledger);
                
                if max_ledger >= end_ledger {
                    // Filter out events beyond end_ledger
                    all_events.retain(|e| e.ledger < end_ledger);
                    break;
                }
            }
            
            // If no cursor, we've reached the end
            if cursor.is_none() {
                break;
            }

            // Update current ledger for the next request
            if let Some(ledger) = last_event_ledger {
                current_ledger = ledger;
            }
        }

        if all_events.is_empty() {
            return Err(BackfillError::NoEventsFound);
        }

        // Sort events by ledger sequence
        all_events.sort_by_key(|e| e.ledger);

        Ok(all_events)
    }

    /// Backfills events with a specific topic filter.
    ///
    /// This allows filtering events by specific topics (e.g., event names).
    pub async fn backfill_events_with_filter(&self, topic_filter: Vec<String>) -> Result<Vec<ContractEvent>> {
        self.config.validate()?;

        let mut all_events = Vec::new();
        let mut cursor: Option<String> = None;
        let mut current_ledger = self.config.start_ledger;

        let filter = EventFilter::for_contract(self.config.contract_id.clone())
            .with_topic(topic_filter);

        loop {
            let request = self.build_request_with_filter(current_ledger, cursor.clone(), filter.clone())?;
            let response = self.send_request(request).await?;

            if response.result.events.is_empty() {
                break;
            }

            // Get the cursor and last ledger before consuming events
            let last_event_ledger = response.result.events.last().and_then(|e| e.ledger.parse().ok());
            cursor = response.result.events.last().map(|e| e.paging_token.clone());

            let events: Vec<ContractEvent> = response
                .result
                .events
                .into_iter()
                .map(|e| self.convert_rpc_event(e))
                .collect();

            all_events.extend(events.clone());

            // Check if we've reached the end ledger
            if let Some(end_ledger) = self.config.end_ledger {
                let max_ledger = all_events
                    .iter()
                    .map(|e| e.ledger)
                    .max()
                    .unwrap_or(current_ledger);
                
                if max_ledger >= end_ledger {
                    all_events.retain(|e| e.ledger < end_ledger);
                    break;
                }
            }
            
            if cursor.is_none() {
                break;
            }

            // Update current ledger for the next request
            if let Some(ledger) = last_event_ledger {
                current_ledger = ledger;
            }
        }

        if all_events.is_empty() {
            return Err(BackfillError::NoEventsFound);
        }

        all_events.sort_by_key(|e| e.ledger);
        Ok(all_events)
    }

    /// Builds the RPC request for fetching events.
    fn build_request(&self, start_ledger: u32, cursor: Option<String>) -> Result<GetEventsRequest> {
        let pagination = cursor.map(|c| Pagination {
            cursor: Some(c),
            limit: self.config.batch_size,
        });

        let params = GetEventsParams {
            start_ledger: start_ledger.to_string(),
            end_ledger: self.config.end_ledger.map(|l| l.to_string()),
            filters: vec![EventFilter::for_contract(self.config.contract_id.clone())],
            pagination,
            xdr_format: Some("json".to_string()),
        };

        Ok(GetEventsRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getEvents".to_string(),
            params,
        })
    }

    /// Builds the RPC request with a custom filter.
    fn build_request_with_filter(
        &self,
        start_ledger: u32,
        cursor: Option<String>,
        filter: EventFilter,
    ) -> Result<GetEventsRequest> {
        let pagination = cursor.map(|c| Pagination {
            cursor: Some(c),
            limit: self.config.batch_size,
        });

        let params = GetEventsParams {
            start_ledger: start_ledger.to_string(),
            end_ledger: self.config.end_ledger.map(|l| l.to_string()),
            filters: vec![filter],
            pagination,
            xdr_format: Some("json".to_string()),
        };

        Ok(GetEventsRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getEvents".to_string(),
            params,
        })
    }

    /// Sends the RPC request to the server.
    async fn send_request(&self, request: GetEventsRequest) -> Result<GetEventsResponse> {
        let response = self
            .client
            .post(&self.config.rpc_url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(BackfillError::RpcError(format!(
                "RPC request failed with status: {}",
                response.status()
            )));
        }

        let rpc_response: GetEventsResponse = response.json().await?;
        Ok(rpc_response)
    }

    /// Converts an RPC event to a ContractEvent.
    fn convert_rpc_event(&self, rpc_event: RpcEvent) -> ContractEvent {
        ContractEvent {
            id: rpc_event.id,
            ledger: rpc_event
                .ledger
                .parse()
                .unwrap_or(0),
            ledger_closed_at: rpc_event.ledger_closed_at,
            contract_id: rpc_event.contract_id.unwrap_or_default(),
            event_type: rpc_event.event_type,
            topics: rpc_event.topic,
            data: rpc_event.value,
            in_successful_contract_call: rpc_event.in_successful_contract_call,
            paging_token: rpc_event.paging_token,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backfill_config_validation() {
        let valid_config = BackfillConfig::new(
            "https://soroban-testnet.stellar.org".to_string(),
            "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
            100000,
        );
        assert!(valid_config.validate().is_ok());

        let invalid_config = BackfillConfig {
            rpc_url: "".to_string(),
            contract_id: "test".to_string(),
            start_ledger: 100000,
            end_ledger: None,
            batch_size: 100,
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_event_filter_creation() {
        let filter = EventFilter::for_contract("test_contract".to_string());
        assert_eq!(filter.contract_ids, vec!["test_contract"]);
        assert_eq!(filter.r#type, Some("contract".to_string()));

        let filter_with_topic = filter.with_topic(vec!["topic1".to_string()]);
        assert_eq!(filter_with_topic.topics.len(), 1);
    }
}
