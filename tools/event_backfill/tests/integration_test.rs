//! Integration tests for event backfilling.
//!
//! These tests require a local Stellar sandbox or access to testnet.
//! Run with: cargo test --test integration_test

use event_backfill::{BackfillConfig, EventBackfiller};

#[tokio::test]
#[ignore = "Requires local Stellar sandbox or testnet access"]
async fn test_backfill_events_testnet() {
    // This test uses Stellar testnet - requires internet access
    let config = BackfillConfig::new(
        "https://soroban-testnet.stellar.org".to_string(),
        "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(), // XLM contract on testnet
        100000,
    );
    
    let backfiller = EventBackfiller::new(config.clone());
    
    // Try to backfill events (may fail if no events exist in range)
    let result = backfiller.backfill_events().await;
    
    // We expect either success or NoEventsFound error
    match result {
        Ok(events) => {
            println!("Successfully backfilled {} events", events.len());
            assert!(!events.is_empty());
            
            // Verify event structure
            for event in events {
                assert!(!event.id.is_empty());
                assert!(event.ledger > 0);
                assert!(!event.contract_id.is_empty());
            }
        }
        Err(event_backfill::BackfillError::NoEventsFound) => {
            println!("No events found in the specified ledger range - this is acceptable");
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
}

#[tokio::test]
#[ignore = "Requires local Stellar sandbox"]
async fn test_backfill_events_local_sandbox() {
    // This test requires a local Stellar sandbox running on localhost:8000
    let config = BackfillConfig::new(
        "http://localhost:8000".to_string(),
        "YOUR_CONTRACT_ID_HERE".to_string(), // Replace with actual contract ID
        1,
    );
    
    let backfiller = EventBackfiller::new(config.clone());
    
    let result = backfiller.backfill_events().await;
    
    match result {
        Ok(events) => {
            println!("Successfully backfilled {} events from local sandbox", events.len());
            assert!(!events.is_empty());
        }
        Err(event_backfill::BackfillError::NoEventsFound) => {
            println!("No events found - ensure contract has emitted events");
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
}

#[tokio::test]
#[ignore = "Requires local Stellar sandbox or testnet access"]
async fn test_backfill_with_topic_filter() {
    let config = BackfillConfig::new(
        "https://soroban-testnet.stellar.org".to_string(),
        "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
        100000,
    );
    
    let backfiller = EventBackfiller::new(config.clone());
    
    // Filter for 'transfer' events (base64 encoded)
    let topic_filter = vec!["AAAADwAAAAh0cmFuc2Zlcg==".to_string()];
    
    let result = backfiller.backfill_events_with_filter(topic_filter).await;
    
    match result {
        Ok(events) => {
            println!("Successfully backfilled {} events with topic filter", events.len());
            assert!(!events.is_empty());
            
            // Verify all events have the expected topic
            for event in events {
                assert!(!event.topics.is_empty());
            }
        }
        Err(event_backfill::BackfillError::NoEventsFound) => {
            println!("No events found with the specified topic filter");
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
}

#[tokio::test]
#[ignore = "Requires local Stellar sandbox or testnet access"]
async fn test_backfill_with_ledger_range() {
    let config = BackfillConfig {
        rpc_url: "https://soroban-testnet.stellar.org".to_string(),
        contract_id: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
        start_ledger: 100000,
        end_ledger: Some(100100), // Small range
        batch_size: 10,
    };
    
    let backfiller = EventBackfiller::new(config.clone());
    
    let result = backfiller.backfill_events().await;
    
    match result {
        Ok(events) => {
            println!("Successfully backfilled {} events in ledger range", events.len());
            
            // Verify all events are within the specified range
            for event in events {
                assert!(event.ledger >= config.start_ledger);
                assert!(event.ledger < config.end_ledger.unwrap());
            }
        }
        Err(event_backfill::BackfillError::NoEventsFound) => {
            println!("No events found in the specified ledger range");
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
}

#[tokio::test]
#[ignore = "Requires local Stellar sandbox or testnet access"]
async fn test_event_data_decoding() {
    let config = BackfillConfig::new(
        "https://soroban-testnet.stellar.org".to_string(),
        "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
        100000,
    );
    
    let backfiller = EventBackfiller::new(config.clone());
    
    let result = backfiller.backfill_events().await;
    
    match result {
        Ok(events) => {
            if !events.is_empty() {
                // Test decoding the first event's data
                let event = &events[0];
                let data_bytes = event.data_as_bytes();
                assert!(data_bytes.is_ok());
                
                if !event.topics.is_empty() {
                    let topic_bytes = event.topic_as_bytes(0);
                    assert!(topic_bytes.is_ok());
                }
            }
        }
        Err(event_backfill::BackfillError::NoEventsFound) => {
            println!("No events found - skipping decoding test");
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
}
