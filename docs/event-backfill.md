# Event Backfill Tool

## Overview

The event backfill tool provides functionality to query historical Soroban contract events from Stellar RPC for backfilling purposes. This is useful for reconstructing state after an SDK outage or for analytics.

## Architecture

The tool uses the **Stellar RPC API's `getEvents` method** (not Horizon) to query Soroban contract events. RPC is the preferred method for Soroban smart contracts, while Horizon is the legacy API for classic Stellar operations.

### Key Components

- **`BackfillConfig`**: Configuration for event backfilling operations
- **`EventBackfiller`**: Main client for querying events
- **`ContractEvent`**: Parsed event structure with metadata
- **`EventFilter`**: Filter for specific event types and topics

## Usage

### Basic Example

```rust
use event_backfill::{EventBackfiller, BackfillConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = BackfillConfig::new(
        "https://soroban-testnet.stellar.org".to_string(),
        "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
        100000,
    );

    let backfiller = EventBackfiller::new(config);
    let events = backfiller.backfill_events().await?;

    println!("Backfilled {} events", events.len());
    Ok(())
}
```

### With Ledger Range

```rust
let config = BackfillConfig {
    rpc_url: "https://soroban-testnet.stellar.org".to_string(),
    contract_id: "CONTRACT_ID".to_string(),
    start_ledger: 100000,
    end_ledger: Some(200000), // Optional end ledger
    batch_size: 100,
};
```

### With Topic Filter

```rust
let backfiller = EventBackfiller::new(config);

// Filter for specific event topics (base64-encoded)
let topic_filter = vec!["AAAADwAAAAh0cmFuc2Zlcg==".to_string()];
let events = backfiller.backfill_events_with_filter(topic_filter).await?;
```

## Configuration

### BackfillConfig Fields

- **`rpc_url`**: Stellar RPC server URL (e.g., `"https://soroban-testnet.stellar.org"`)
- **`contract_id`**: Contract ID to filter events for
- **`start_ledger`**: Starting ledger sequence (inclusive)
- **`end_ledger`**: Optional ending ledger sequence (exclusive). If None, fetches to latest
- **`batch_size`**: Number of events to fetch per batch (max 10000 per RPC limits)

### RPC Endpoints

- **Testnet**: `https://soroban-testnet.stellar.org`
- **Mainnet**: `https://soroban.stellar.org`
- **Local Sandbox**: `http://localhost:8000`

## Event Structure

### ContractEvent

```rust
pub struct ContractEvent {
    pub id: String,
    pub ledger: u32,
    pub ledger_closed_at: String,
    pub contract_id: String,
    pub event_type: String,
    pub topics: Vec<String>,
    pub data: String,
    pub in_successful_contract_call: bool,
    pub paging_token: String,
}
```

### Decoding Event Data

Event data and topics are returned as base64-encoded strings. You can decode them to bytes:

```rust
let data_bytes = event.data_as_bytes()?;
let topic_bytes = event.topic_as_bytes(0)?;
```

## Error Handling

The tool uses a custom `BackfillError` type:

- **`RpcError`**: Error from the RPC client
- **`HttpError`**: HTTP request error
- **`JsonError`**: JSON parsing error
- **`InvalidConfig`**: Invalid configuration
- **`NoEventsFound`**: No events found for the given criteria
- **`PaginationError`**: Pagination error
- **`XdrError`**: XDR decoding error

## Testing

### Unit Tests

Run unit tests (no network required):

```bash
cargo test -p event_backfill
```

### Integration Tests

Integration tests require access to a Stellar RPC server:

```bash
# Run all integration tests (requires testnet access)
cargo test -p event_backfill --test integration_test

# Run specific integration test
cargo test -p event_backfill --test integration_test test_backfill_events_testnet
```

Note: Integration tests are marked with `#[ignore]` by default. Remove the attribute or run with:

```bash
cargo test -p event_backfill --test integration_test -- --ignored
```

## Limitations

1. **RPC Retention**: Stellar RPC typically retains events for 7 days. For longer history, consider using an indexer like Mercury, SubQuery, or Goldsky.

2. **Batch Size**: Maximum batch size is 10,000 events per request (RPC limit).

3. **Rate Limiting**: Be mindful of RPC rate limits when backfilling large ranges.

4. **Contract ID Limit**: Maximum 5 contract IDs per filter.

## Use Cases

### State Reconstruction After Outage

```rust
// Backfill events from the last known ledger
let config = BackfillConfig::new(
    rpc_url,
    contract_id,
    last_known_ledger,
);

let backfiller = EventBackfiller::new(config);
let events = backfiller.backfill_events().await?;

// Reconstruct state from events
for event in events {
    // Process event to update local state
}
```

### Analytics

```rust
// Backfill events for analytics
let config = BackfillConfig {
    rpc_url: "https://soroban-mainnet.stellar.org".to_string(),
    contract_id: contract_id,
    start_ledger: analytics_start_ledger,
    end_ledger: Some(current_ledger),
    batch_size: 1000,
};

let events = EventBackfiller::new(config).backfill_events().await?;

// Analyze event patterns
analyze_events(&events);
```

## Best Practices

1. **Use Appropriate Batch Sizes**: Start with smaller batch sizes (100-500) for testing, increase for production.

2. **Handle Pagination**: The tool handles pagination automatically, but be aware of rate limits.

3. **Validate Configuration**: Always call `config.validate()` before creating a backfiller.

4. **Error Handling**: Handle `NoEventsFound` gracefully - it's expected when no events exist in the range.

5. **Ledger Ranges**: Use reasonable ledger ranges to avoid timeouts and rate limit issues.

## Troubleshooting

### No Events Found

- Verify the contract ID is correct
- Check that events exist in the specified ledger range
- Ensure the RPC server has data for the requested range

### RPC Errors

- Verify the RPC URL is accessible
- Check network connectivity
- Ensure the RPC server is operational

### Timeout Errors

- Reduce the batch size
- Use smaller ledger ranges
- Check network latency to the RPC server

## References

- [Stellar RPC getEvents Documentation](https://developers.stellar.org/docs/data/apis/rpc/api-reference/methods/getEvents)
- [Soroban Events Guide](https://developers.stellar.org/docs/learn/fundamentals/stellar-data-structures/events)
- [Event Ingestion Guide](https://developers.stellar.org/docs/build/guides/events/ingest)
