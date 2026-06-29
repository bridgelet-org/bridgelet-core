//! Unit tests for event backfilling.

use crate::types::{BackfillConfig, ContractEvent, EventFilter};

#[test]
fn test_backfill_config_valid() {
    let config = BackfillConfig::new(
        "https://soroban-testnet.stellar.org".to_string(),
        "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
        100000,
    );
    assert!(config.validate().is_ok());
}

#[test]
fn test_backfill_config_empty_rpc_url() {
    let config = BackfillConfig {
        rpc_url: "".to_string(),
        contract_id: "test".to_string(),
        start_ledger: 100000,
        end_ledger: None,
        batch_size: 100,
    };
    assert!(config.validate().is_err());
}

#[test]
fn test_backfill_config_empty_contract_id() {
    let config = BackfillConfig {
        rpc_url: "https://test.com".to_string(),
        contract_id: "".to_string(),
        start_ledger: 100000,
        end_ledger: None,
        batch_size: 100,
    };
    assert!(config.validate().is_err());
}

#[test]
fn test_backfill_config_invalid_batch_size_zero() {
    let config = BackfillConfig {
        rpc_url: "https://test.com".to_string(),
        contract_id: "test".to_string(),
        start_ledger: 100000,
        end_ledger: None,
        batch_size: 0,
    };
    assert!(config.validate().is_err());
}

#[test]
fn test_backfill_config_invalid_batch_size_too_large() {
    let config = BackfillConfig {
        rpc_url: "https://test.com".to_string(),
        contract_id: "test".to_string(),
        start_ledger: 100000,
        end_ledger: None,
        batch_size: 10001,
    };
    assert!(config.validate().is_err());
}

#[test]
fn test_backfill_config_end_ledger_before_start() {
    let config = BackfillConfig {
        rpc_url: "https://test.com".to_string(),
        contract_id: "test".to_string(),
        start_ledger: 100000,
        end_ledger: Some(99999),
        batch_size: 100,
    };
    assert!(config.validate().is_err());
}

#[test]
fn test_backfill_config_end_ledger_equal_start() {
    let config = BackfillConfig {
        rpc_url: "https://test.com".to_string(),
        contract_id: "test".to_string(),
        start_ledger: 100000,
        end_ledger: Some(100000),
        batch_size: 100,
    };
    assert!(config.validate().is_err());
}

#[test]
fn test_backfill_config_valid_end_ledger() {
    let config = BackfillConfig {
        rpc_url: "https://test.com".to_string(),
        contract_id: "test".to_string(),
        start_ledger: 100000,
        end_ledger: Some(200000),
        batch_size: 100,
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_event_filter_for_contract() {
    let filter = EventFilter::for_contract("test_contract".to_string());
    assert_eq!(filter.contract_ids, vec!["test_contract"]);
    assert_eq!(filter.r#type, Some("contract".to_string()));
    assert!(filter.topics.is_empty());
}

#[test]
fn test_event_filter_with_topic() {
    let filter = EventFilter::for_contract("test_contract".to_string())
        .with_topic(vec!["topic1".to_string()]);
    assert_eq!(filter.contract_ids, vec!["test_contract"]);
    assert_eq!(filter.topics.len(), 1);
    assert_eq!(filter.topics[0], vec!["topic1"]);
}

#[test]
fn test_event_filter_multiple_topics() {
    let filter = EventFilter::for_contract("test_contract".to_string())
        .with_topic(vec!["topic1".to_string()])
        .with_topic(vec!["topic2".to_string()]);
    assert_eq!(filter.topics.len(), 2);
}

#[test]
fn test_contract_event_creation() {
    let event = ContractEvent {
        id: "test-id".to_string(),
        ledger: 100000,
        ledger_closed_at: "2024-01-01T00:00:00Z".to_string(),
        contract_id: "test-contract".to_string(),
        event_type: "contract".to_string(),
        topics: vec!["topic1".to_string()],
        data: "data".to_string(),
        in_successful_contract_call: true,
        paging_token: "token".to_string(),
    };
    assert_eq!(event.id, "test-id");
    assert_eq!(event.ledger, 100000);
}

#[test]
fn test_contract_event_topic_out_of_bounds() {
    let event = ContractEvent {
        id: "test-id".to_string(),
        ledger: 100000,
        ledger_closed_at: "2024-01-01T00:00:00Z".to_string(),
        contract_id: "test-contract".to_string(),
        event_type: "contract".to_string(),
        topics: vec![],
        data: "data".to_string(),
        in_successful_contract_call: true,
        paging_token: "token".to_string(),
    };
    let result = event.topic_as_bytes(0);
    assert!(result.is_err());
}

#[test]
fn test_backfill_config_serialization() {
    let config = BackfillConfig::new(
        "https://soroban-testnet.stellar.org".to_string(),
        "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
        100000,
    );
    
    let json = serde_json::to_string(&config).unwrap();
    let parsed: BackfillConfig = serde_json::from_str(&json).unwrap();
    
    assert_eq!(parsed.rpc_url, config.rpc_url);
    assert_eq!(parsed.contract_id, config.contract_id);
    assert_eq!(parsed.start_ledger, config.start_ledger);
}

#[test]
fn test_contract_event_serialization() {
    let event = ContractEvent {
        id: "test-id".to_string(),
        ledger: 100000,
        ledger_closed_at: "2024-01-01T00:00:00Z".to_string(),
        contract_id: "test-contract".to_string(),
        event_type: "contract".to_string(),
        topics: vec!["topic1".to_string()],
        data: "data".to_string(),
        in_successful_contract_call: true,
        paging_token: "token".to_string(),
    };
    
    let json = serde_json::to_string(&event).unwrap();
    let parsed: ContractEvent = serde_json::from_str(&json).unwrap();
    
    assert_eq!(parsed.id, event.id);
    assert_eq!(parsed.ledger, event.ledger);
    assert_eq!(parsed.contract_id, event.contract_id);
}

#[test]
fn test_event_filter_serialization() {
    let filter = EventFilter::for_contract("test_contract".to_string());
    
    let json = serde_json::to_string(&filter).unwrap();
    let parsed: EventFilter = serde_json::from_str(&json).unwrap();
    
    assert_eq!(parsed.contract_ids, filter.contract_ids);
    assert_eq!(parsed.r#type, filter.r#type);
    // Topics field is skipped when empty due to skip_serializing_if
    assert!(parsed.topics.is_empty());
}
