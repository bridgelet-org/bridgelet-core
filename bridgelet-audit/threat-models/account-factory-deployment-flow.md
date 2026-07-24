# Threat Model: Account Factory Deployment Flow

## Overview

The Account Factory contract enables batch deployment of Ephemeral Account contracts with deterministic addresses. This threat model analyzes the trust assumptions, access control, and security properties of the deployment flow.

## Contract Entry Points

### `initialize(env, ephemeral_account_wasm_hash)`

**Who can call:** Anyone (no access control)

**Capability granted:** Controls which WASM bytecode is deployed for all subsequent ephemeral accounts

**Trust assumptions:**

- The caller must be trusted to set a valid, audited ephemeral account contract WASM hash
- Once set, the WASM hash cannot be changed without redeploying the factory
- A malicious WASM hash could compromise all subsequently deployed accounts

**Threats:**

- **Unauthorized WASM replacement:** An attacker who calls `initialize()` before the legitimate operator can replace the WASM hash with a malicious contract
- **Front-running:** If the factory deployment transaction is visible in the mempool, an attacker could call `initialize()` before the legitimate initialization
- **No upgrade path:** Once initialized, there is no mechanism to update the WASM hash if a critical bug is discovered

**Mitigations:**

- Deploy and call `initialize()` atomically in the same transaction
- Use a privileged deployer that restricts who can call `initialize()` (currently not implemented)
- Monitor the factory for unexpected initialization events

### `batch_initialize(env, creator, requests)`

**Who can call:** Any address that can satisfy `creator.require_auth()`

**Capability granted:** Deploys multiple ephemeral accounts and sets their initial configuration

**Trust assumptions:**

- The `creator` address is authorized to create accounts on behalf of users
- The creator will set appropriate `expiry_ledger` and `recovery_address` values
- The creator is trusted to be set as both `authorized_controller` and `admin` for all deployed accounts

**Threats:**

- **Unauthorized account creation:** If the creator's private key is compromised, an attacker could create malicious accounts
- **Misconfiguration:** The creator could set inappropriate expiry times or recovery addresses
- **Centralization:** All accounts in a batch share the same creator, authorized_controller, and admin, creating a single point of control

**Mitigations:**

- Use hardware security modules (HSMs) or multi-sig for the creator address
- Implement off-chain validation of request parameters
- Consider allowing per-account customization of controller/admin addresses

## Deterministic Address Derivation

### Salt Construction

**Current implementation (lines 55-57):**

```rust
let mut salt_bytes = [0u8; 32];
salt_bytes[28..32].copy_from_slice(&(index as u32).to_be_bytes());
let salt = BytesN::from_array(&env, &salt_bytes);
```

**Properties:**

- Salt is 32 bytes, with only the last 4 bytes (28..32) varying based on the request index
- First 28 bytes are all zeros
- Address derivation: `env.deployer().with_current_contract(salt).deploy_v2(wasm_hash, ())`

**Trust assumptions:**

- The factory contract address is stable and predictable
- The salt derivation produces unique addresses for each index in a batch
- The same (creator, index) combination always produces the same address

**Threats:**

- **Salt collision:** If two different batches use overlapping index ranges, they could produce the same addresses
- **Predictability:** The simple salt derivation makes addresses trivially predictable, which could facilitate front-running
- **Limited entropy:** Only 4 bytes of entropy (32-bit index) limits the number of unique addresses per batch to ~4 billion
- **Zero prefix:** The 28-byte zero prefix is wasteful and reduces the effective address space

**Required salt properties for intended behavior:**

1. **Uniqueness:** Each salt must produce a unique contract address within the factory's lifetime
2. **Determinism:** The same input parameters must always produce the same address
3. **Unpredictability:** Addresses should not be easily guessable by external observers
4. **Batch isolation:** Different batches should not produce overlapping address ranges
5. **Sufficient entropy:** Salt should have enough randomness to support the expected number of deployments

**Recommended salt improvements:**
- Include creator address in the salt to ensure batch isolation
- Add a timestamp or nonce to prevent predictability
- Use a cryptographic hash of (creator, timestamp, index) instead of simple index encoding
- Consider using a monotonically increasing counter stored in factory storage

## Authorized Controller and Admin Relationship

### Current Implementation

**Lines 70-71:**

```rust
&creator,  // authorized_controller
&creator,  // admin
```

**Who sets them:** The Account Factory contract, using the `creator` parameter from `batch_initialize()`

**Trust assumptions:**
- The creator is trusted to act as the authorized_controller for sweep operations
- The creator is trusted to perform contract upgrades as the admin
- The creator will maintain the security of their private key
- The creator will not abuse their control over user accounts

**Capabilities granted:**

**Authorized Controller:**

- Can invoke `sweep()` on behalf of users (with signature verification)
- Can invoke `sweep_claim()` without signature (direct claim path)
- Controls when funds can be swept from ephemeral accounts

**Admin:**

- Can upgrade the ephemeral account contract WASM
- Can potentially introduce backdoors or bugs through upgrades
- Has permanent control over the account's code

**Threats:**
- **Single point of failure:** Compromise of the creator's key compromises all accounts in the batch
- **Upgrade abuse:** A malicious admin could upgrade accounts to a malicious WASM
- **Sweep interference:** The controller could refuse to process legitimate sweep claims
- **Censorship:** The controller could selectively block sweep operations for specific users
- **Rug pull potential:** The creator could sweep all accounts to themselves if signature verification is weak

**Mitigations:**

- Use a decentralized controller (e.g., a multi-sig or DAO)
- Implement time-locks on upgrade operations
- Allow users to specify their own controller/admin addresses
- Implement emergency recovery mechanisms that don't require controller cooperation
- Consider using a separate controller for each account or group of accounts

## Deployment Flow Security Properties

### Atomicity

**Current behavior:** Each account deployment is independent; failure of one deployment does not affect others

**Threats:**

- **Partial deployment:** Some accounts in a batch may succeed while others fail
- **Inconsistent state:** Users may receive inconsistent results from the same batch operation

**Mitigations:**
- Implement all-or-nothing semantics using defensive programming
- Provide clear error reporting for failed deployments
- Allow retry of failed individual deployments

### Authorization Propagation

**Current behavior:** The factory calls `initialize()` on each deployed account as the factory contract itself

**Trust assumptions:**

- The factory contract is trusted to initialize accounts on behalf of the creator
- The creator's authorization is sufficient to authorize the factory to act on their behalf

**Threats:**
- **Authorization confusion:** Users may not understand that the factory is initializing accounts, not the creator directly
- **Privilege escalation:** If the factory is compromised, it could initialize accounts with malicious parameters

**Mitigations:**

- Clearly document the authorization flow
- Consider having the creator directly initialize each account (though this increases gas costs)
- Implement factory-level access controls

## Recommendations

### High Priority

1. **Add access control to `initialize()`:** Restrict who can set the WASM hash to a trusted deployer
2. **Improve salt derivation:** Use creator-specific salts with cryptographic hashing to prevent collisions and improve unpredictability
3. **Separate controller and admin:** Allow different addresses for controller and admin roles to reduce single points of failure
4. **Implement upgrade safeguards:** Add time-locks or multi-sig requirements for admin upgrades

### Medium Priority

1. **Add batch isolation:** Include batch-specific identifiers (timestamp, nonce) in salt derivation
2. **Implement monitoring:** Add events to track factory initialization and batch deployments
3. **Consider per-account configuration:** Allow requests to specify custom controller/admin addresses
4. **Add emergency recovery:** Implement mechanisms for users to recover accounts if the controller is unresponsive

### Low Priority

1. **Improve error reporting:** Return detailed error information in `AccountInitResult`
2. **Add rate limiting:** Prevent abuse of batch initialization through rate limits or fees
3. **Implement factory upgrade mechanism:** Allow the factory itself to be upgraded safely
4. **Add pausability:** Allow the factory to be paused in case of emergencies

## Conclusion

The current Account Factory implementation has significant trust assumptions centered on the `creator` address and the initial `initialize()` caller. The deterministic address derivation using simple index-based salts is predictable and could lead to collisions. The concentration of controller and admin roles in a single address creates a single point of failure.

For mainnet deployment, the following should be addressed:

- Secure the factory initialization process
- Improve salt derivation for better security properties
- Consider separating controller and admin roles
- Implement monitoring and emergency recovery mechanisms
