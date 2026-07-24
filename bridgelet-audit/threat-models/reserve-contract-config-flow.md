# Threat Model: Reserve Contract Configuration Flow

## Overview

The Reserve Contract is a focused on-chain contract that stores and exposes the base reserve configuration for the Bridgelet system. It provides a single source of truth for the Stellar network's base reserve requirement, which ephemeral accounts need to track user payments versus network overhead. This threat model analyzes the trust assumptions around who can set this system-wide value and the implications of incorrect configuration.

## Contract Entry Points

### `initialize(env, admin)`

**Who can call:** Anyone (no access control on the function itself, but requires admin authorization)

**Capability granted:** Sets the admin address that has exclusive control over `set_base_reserve()`

**Trust assumptions:**

- The initial caller is trusted to set a legitimate admin address
- The admin address will be securely managed (e.g., multi-sig, HSM)
- Once set, the admin cannot be changed without redeploying the contract

**Threats:**

- **Unauthorized initialization:** An attacker who calls `initialize()` before the legitimate operator can set themselves as the admin
- **Front-running:** If the deployment transaction is visible in the mempool, an attacker could initialize with a malicious admin
- **Admin key compromise:** If the admin private key is compromised, an attacker could set arbitrary reserve values
- **No admin rotation:** Once initialized, there is no mechanism to change the admin if the key is lost or compromised

**Mitigations:**

- Deploy and call `initialize()` atomically in the same transaction
- Use a multi-sig or DAO as the admin address to distribute trust
- Store the admin address securely using hardware security modules (HSMs)
- Consider implementing an admin rotation mechanism for long-term operability

### `set_base_reserve(env, amount)`

**Who can call:** Only the admin address set during `initialize()`

**Capability granted:** Sets the system-wide base reserve value used by downstream contracts

**Trust assumptions:**

- The admin will set the correct base reserve value matching the Stellar network's current requirement
- The admin will not maliciously set an incorrect value to disrupt operations
- The admin will update the value promptly when Stellar changes its base reserve
- The admin understands the difference between XLM and stroops (1 XLM = 10,000,000 stroops)

**Bounds checking (lines 103-108):**

```rust
if amount <= 0 {
    return Err(Error::InvalidAmount);
}
if amount > MAX_RESERVE_STROOPS {
    return Err(Error::AmountTooLarge);
}
```

**Current bounds:**
- Minimum: Greater than 0 stroops
- Maximum: `MAX_RESERVE_STROOPS` = 100,000,000,000 stroops (10,000 XLM)
- Purpose: Catches operator mistakes (e.g., passing value in XLM instead of stroops)

**Threats:**

- **Incorrect value:** Admin sets a value that doesn't match the actual Stellar network base reserve
- **Value too low:** Underestimating the reserve could cause ephemeral accounts to incorrectly calculate user funds as available
- **Value too high:** Overestimating the reserve could cause ephemeral accounts to incorrectly withhold funds from users
- **Unit confusion:** Admin accidentally sets value in XLM instead of stroops (mitigated by MAX_RESERVE_STROOPS ceiling)
- **Stale value:** Admin fails to update the value when Stellar changes its base reserve
- **Malicious value:** Admin intentionally sets an incorrect value to steal funds or disrupt operations

**Mitigations:**

- Implement off-chain validation with multiple signers
- Add event monitoring to detect unexpected changes
- Use time-locks or multi-step confirmation for value changes
- Implement automated monitoring that alerts when the contract value diverges from on-chain network parameters
- Consider requiring a governance process for changes
- Add a "proposed value" delay period before changes take effect

## Downstream System Impact

If the base reserve value were incorrect and actually consulted by downstream systems, the following would be affected:

### Ephemeral Account Contracts

**Current relationship:** The ephemeral account contract (`contracts/ephemeral_account/src/lib.rs`) currently uses a hardcoded constant:

```rust
const BASE_RESERVE_STROOPS: i128 = 1_000_000_000;
```

**If it consulted the Reserve Contract:**

1. **Payment calculation errors:**
   - **Value too low:** Ephemeral accounts would overestimate available user funds. When sweeping, they might attempt to transfer more XLM than actually exists, causing transaction failures or partial sweeps
   - **Value too high:** Ephemeral accounts would underestimate available user funds. Users would receive less than their full payment amount, with the excess being incorrectly classified as reserve

2. **Reserve reclamation errors:**
   - **Value too low:** The contract would attempt to reclaim less reserve than actually required, leaving dust accounts that cannot be closed
   - **Value too high:** The contract would attempt to reclaim more reserve than exists, potentially causing transaction failures or incorrect balance calculations

3. **Account lifecycle issues:**
   - Incorrect reserve values could cause accounts to become unsustainable (unable to pay fees) or prevent proper closure
   - The `reclaim_reserve_to()` function would miscalculate how much can be reclaimed, leading to incomplete reserve recovery

4. **Economic attacks:**
   - A malicious admin could set the reserve to 1 stroop, allowing sweep operations to drain almost all funds including network-required reserves
   - A malicious admin could set the reserve to the maximum, causing sweep operations to fail and funds to be locked in expired accounts

### Account Factory

**Current relationship:** The Account Factory does not currently reference the Reserve Contract. It deploys ephemeral accounts with their own hardcoded reserve values.

**If it consulted the Reserve Contract:**

1. **Deployment failures:** If the reserve value is not set or is invalid, batch initialization could fail
2. **Inconsistent configurations:** Different batches deployed at different times could have different effective reserve values if the value changes between deployments
3. **Predictability issues:** Address derivation might be affected if reserve configuration becomes part of the initialization parameters

### Sweep Controller

**Current relationship:** The Sweep Controller does not currently reference the Reserve Contract.

**If it consulted the Reserve Contract:**

1. **Sweep calculation errors:** The controller would use incorrect values when estimating sweep amounts or fees
2. **Validation failures:** Pre-sweep validation could incorrectly reject valid sweeps or accept invalid ones
3. **Fee estimation errors:** Transaction fee calculations would be incorrect, potentially causing failed transactions or overpayment

### Off-Chain Systems

**Impact on monitoring and observability:**

- Dashboards displaying account balances would show incorrect values
- Alerting systems might trigger false positives or miss actual issues
- Accounting systems would have incorrect records of user funds vs. network overhead

**Impact on user experience:**

- Users might receive incorrect payment amounts
- Sweep operations might fail unexpectedly
- Support tickets would increase due to confusing balance discrepancies

## Current System Relationship

### Integration Status

**Reserve Contract integration:**

- The Reserve Contract exists and is functional
- It provides `get_base_reserve()`, `require_base_reserve()`, and `has_base_reserve()` functions
- However, the ephemeral account contract does **not** currently call this contract
- The ephemeral account uses a hardcoded `BASE_RESERVE_STROOPS` constant instead

**Implications:**

- The Reserve Contract is currently **not** a security-critical component because it is not consulted by active contracts
- Changing the value in the Reserve Contract would have **no effect** on the current system
- The system operates as if the Reserve Contract does not exist from a functional perspective

**Why this matters for threat modeling:**

- Even though currently unused, the contract represents a **planned integration point**
- If integration is added in the future, the threat model becomes immediately relevant
- The current lack of integration is a **defense in depth** - even if the Reserve Contract were compromised, it would not affect active operations
- However, this also means there is no **operational testing** of the Reserve Contract's security properties

### Deployment Considerations

**Current deployment state:**

- The Reserve Contract may or may not be deployed to the network
- If deployed, it may or may not be initialized
- If initialized, the base reserve value may or may not be set

**Operational risks:**

- If the contract is deployed but not initialized, anyone could initialize it and become the admin
- If the contract is deployed and initialized with a test admin, that admin might have less security than production requires
- The contract's TTL (time to live) needs to be managed to prevent expiration if it's deployed but unused

**Recommendations for current state:**

- If not planning immediate integration, consider **not deploying** the Reserve Contract to mainnet
- If deployed for testing, ensure it uses a testnet-specific admin address
- Monitor the contract for unexpected initialization events
- Document the integration plan so future developers understand the intended relationship

## Recommendations

### High Priority (if integrating)

1. **Secure admin setup:** Use a multi-sig or DAO as the admin address, not a single private key
2. **Implement value validation:** Add off-chain validation that checks against Stellar's actual network parameters
3. **Add change monitoring:** Implement monitoring that alerts when the reserve value changes
4. **Implement time-locks:** Add a delay between setting a new value and it taking effect

### Medium Priority (if integrating)

1. **Add governance process:** Require a formal governance process for changing the reserve value
2. **Implement admin rotation:** Add a mechanism to rotate the admin key without redeployment
3. **Add value history:** Store historical values to enable auditing and rollback
4. **Implement circuit breakers:** Add automatic checks that prevent obviously incorrect values

### Low Priority (current state)

1. **Decide on deployment:** Determine whether to deploy the contract before integration
2. **Document integration plan:** Clearly document when and how integration will occur
3. **Add integration tests:** Create tests that verify the integration when it's implemented
4. **Plan migration strategy:** Design the migration path from hardcoded values to contract-based values

## Conclusion

The Reserve Contract implements a simple but critical function: storing the system-wide base reserve value. The current implementation has strong access control (admin-only) and bounds checking (0 < amount ≤ 100,000,000,000 stroops). However, the security of the entire system depends on:

1. The admin address being securely managed
2. The admin setting correct values that match Stellar's actual base reserve
3. The admin promptly updating the value when Stellar changes its requirements

**Critical observation:** The Reserve Contract is currently **not integrated** with the ephemeral account contract, which uses a hardcoded constant instead. This means:
- Current security impact is minimal (the contract is effectively unused)
- Future integration would immediately elevate this contract to security-critical status
- The lack of integration represents both a risk (no operational testing) and a benefit (defense in depth)

For mainnet deployment, the decision to integrate the Reserve Contract should be made explicitly, with appropriate security measures implemented before or during integration.
