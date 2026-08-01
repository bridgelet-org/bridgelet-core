# Bridgelet Runbooks

This directory contains runbooks detailing current behavior, operational workarounds, and validation steps for the Bridgelet Core deployment. Note that these describe current operations and do not propose code changes.

## Routine Operations
* [Rotating `ReserveContract`'s Admin Key](./reserve-contract-admin-key-rotation.md): Use when the admin address for a `ReserveContract` needs to be updated or rotated.
* [Rotating `SweepController`'s `authorized_signer` Key](./rotate-authorized-signer-key.md): Use when you need to rotate the off-chain public key used for sweep authorizations.

## Incident-Response
* [Diagnosing a Failed `execute_sweep` or `claim` Call](./diagnose-failed-sweep.md): Use when a sweep or claim transaction fails unexpectedly and requires troubleshooting.
* [Emergency Destination Lock](./emergency-destination-lock.md): Use during an emergency to lock or verify the sweep destination of the `SweepController`.
* [Bulk TTL Extension Sweep](./post-incident-ttl-extension-sweep.md): Use to prevent contract storage expiration across all deployed contracts (e.g., post-incident or during maintenance).
* [Restoring an Archived (TTL-Expired) Contract Instance](./restore-archived-instance-storage.md): Use when a contract instance has expired and its Soroban state needs to be restored.
* [Triaging an Incoming Security Disclosure](./security-disclosure-triage.md): Use when a suspected vulnerability in Bridgelet Core is reported and requires classification.
* [Stuck / Expired Account Fund Recovery](./stuck-expired-account-recovery.md): Use to manually recover funds from accounts that are stuck or expired.

## Pre-Flight & Validation Procedures
* [Cross-Checking ReserveContract's Configured Value](./cross-check-reserve-contract-vs-hardcoded-reserve.md): Use to validate that a deployed reserve contract matches the expected hardcoded base reserve.
* [Validating `batch_initialize` Salt Uniqueness](./validate-batch-initialize-salt-uniqueness.md): Use before calling `AccountFactory::batch_initialize` in production to prevent address collisions.
* [Verifying Nonce State Before Trusting Lock](./verify-claim-vs-execute-sweep-nonce-state.md): Use to confirm whether the `SweepController`'s destination lock is still in force after sweeps.
* [Verifying WASM Hash Before Invoking `upgrade()`](./verify-upgrade-wasm-hash.md): Use to verify the new WASM hash prior to executing a contract upgrade.
