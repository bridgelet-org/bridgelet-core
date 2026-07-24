use soroban_sdk::contracttype;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageKey {
    Initialized,
    Creator,
    ExpiryLedger,
    RecoveryAddress,
    Payments,
    Status,
    SweptTo,
    BaseReserveRemaining,
    AvailableReserve,
    ReserveReclaimed,
    LastSweepId,
    ReserveEventCount,
    LastReserveEvent,
    AuthorizedController,
    Admin,
    AuthorizedSigner,
    SweepNonce,
    AuthorizedDestination,
    EphemeralAccountWasmHash,
    StorageVersion,
}
