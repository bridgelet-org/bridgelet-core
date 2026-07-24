use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// `initialize` was called more than once, or after the factory was
    /// already configured. Without this guard, any caller could overwrite
    /// `EphemeralAccountWasmHash` with a malicious contract wasm (issue #240).
    AlreadyInitialized = 1,
    /// A factory entry point was invoked before `initialize` succeeded.
    /// Currently only reachable from `batch_initialize` when invoked without a
    /// prior `initialize`, since the factory uses a hard `unwrap()` on the
    /// stored wasm hash so the contract decides to panic rather than silently
    /// skip the batch.
    NotInitialized = 2,
}
