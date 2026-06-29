//! # Event Backfill Tool
//!
//! This crate provides functionality to query historical Soroban contract events
//! from Stellar RPC for backfilling purposes. This is useful for reconstructing
//! state after an SDK outage or for analytics.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use event_backfill::{EventBackfiller, BackfillConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = BackfillConfig {
//!         rpc_url: "https://soroban-testnet.stellar.org".to_string(),
//!         contract_id: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
//!         start_ledger: 100000,
//!         end_ledger: None,
//!         batch_size: 100,
//!     };
//!
//!     let backfiller = EventBackfiller::new(config);
//!     let events = backfiller.backfill_events().await?;
//!
//!     println!("Backfilled {} events", events.len());
//!     Ok(())
//! }
//! ```

mod error;
mod types;
mod backfiller;

#[cfg(test)]
mod tests;

pub use error::{BackfillError, Result};
pub use types::{BackfillConfig, ContractEvent, EventFilter};
pub use backfiller::EventBackfiller;
