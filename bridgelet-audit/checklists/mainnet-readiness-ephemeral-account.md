# Pre-Mainnet Checklist: Ephemeral Account Contract

## Signature Verification

- [ ] **Confirm signature verification implementation**: The `verify_sweep_authorization` function (lines 502-510) currently only checks controller authorization via `controller.require_auth()`. Is this the intended security model, or should cryptographic signature verification be implemented?
- [ ] **Validate signature format**: If cryptographic signatures are used, confirm the signature format (e.g., Ed25519, ECDSA) matches the off-chain signing system
- [ ] **Test signature replay protection**: Verify that signatures cannot be replayed across different transactions or accounts
- [ ] **Confirm signature expiration**: If signatures include timestamps, verify they are validated against current ledger time

## Record Payment Authorization

- [ ] **Confirm record_payment authorization model**: The `record_payment` function (lines 83-128) has no explicit authorization check beyond initialization. Should this function be restricted to specific callers (e.g., authorized_controller, admin)?
- [ ] **Validate payment source restrictions**: Should payments only be accepted from specific addresses or contracts?
- [ ] **Test payment limit enforcement**: Confirm the 10-asset payment limit (line 101) is appropriate for mainnet usage patterns
- [ ] **Verify duplicate asset detection**: Test that duplicate asset payments are correctly rejected (line 95-97)

## TTL Extension

- [ ] **Confirm TTL extension requirements**: The contract does not currently expose a function to extend the expiry_ledger. Is this intentional, or should an extension mechanism be added?
- [ ] **Define extension authorization**: If TTL extension is needed, specify who should be authorized to extend (creator, admin, authorized_controller, or permissionless)
- [ ] **Set extension limits**: If TTL extension is implemented, define maximum extensions or absolute expiry limits
- [ ] **Test extension edge cases**: Verify behavior when extending near or past current expiry

## Reserve Source Configuration

- [ ] **Confirm reserve source matches operator intent**: The contract uses a hardcoded `BASE_RESERVE_STROOPS` constant (line 19: 1,000,000,000 stroops). Should this be:
  - Hardcoded (current implementation)
  - Configurable via ReserveContract
  - Set during initialization
  - Dynamically fetched from Stellar network parameters
- [ ] **Validate reserve amount accuracy**: Confirm 1,000,000,000 stroops (100 XLM) matches the current Stellar base reserve requirement
- [ ] **Test reserve reclaim logic**: Verify `reclaim_reserve_to` (lines 512-560) correctly handles partial and full reclaims
- [ ] **Confirm reserve destination**: Validate that reclaimed reserves are sent to the correct destination (sweep destination or recovery address)

## Initialization Parameters

- [ ] **Validate creator address**: Confirm the creator address is set correctly and cannot be modified after initialization
- [ ] **Verify expiry_ledger validation**: Test that expiry_ledger must be in the future (line 52-55)
- [ ] **Confirm recovery_address**: Validate the recovery_address is set correctly and used in expire/recover flows
- [ ] **Test authorized_controller**: Verify the authorized_controller is properly enforced in sweep operations
- [ ] **Validate admin address**: Confirm the admin address is set correctly for upgrade operations

## State Transition Validation

- [ ] **Test all status transitions**: Verify all valid state transitions:
  - Active → PaymentReceived (first payment)
  - PaymentReceived → Swept (sweep/sweep_claim)
  - Active/PaymentReceived → Expired (expire/recover)
- [ ] **Test invalid state transitions**: Confirm invalid transitions are rejected (e.g., Swept → PaymentReceived)
- [ ] **Verify reentrancy protection**: Confirm status updates happen before external operations (line 174)
- [ ] **Test sweep_id tracking**: Verify sweep_id is correctly set and used for reserve reclamation

## Access Control

- [ ] **Verify sweep authorization**: Test that only authorized_controller can invoke sweep (line 507-508)
- [ ] **Verify sweep_claim authorization**: Test that only authorized_controller can invoke sweep_claim (line 220-221)
- [ ** **Verify recover authorization**: Test that only creator or recovery_address can invoke recover (lines 405-408)
- [ ] **Verify expire permissionless**: Confirm expire() is intentionally permissionless (line 285-286)
- [ ] **Verify upgrade authorization**: Test that only admin can invoke upgrade (lines 427-428)

## Error Handling

- [ ] **Test all error conditions**: Verify all Error variants are properly triggered:
  - AlreadyInitialized
  - NotInitialized
  - InvalidExpiry
  - InvalidAmount
  - DuplicateAsset
  - TooManyPayments
  - AlreadySwept
  - NoPaymentReceived
  - AccountExpired
  - Unauthorized
  - InvalidStatus
  - NotExpired
  - NotUpgradeAdmin
- [ ] **Validate error messages**: Confirm error messages are clear and actionable for debugging
- [ ] **Test edge cases**: Verify behavior with zero amounts, negative values, and overflow conditions

## Event Emission

- [ ] **Verify AccountCreated event**: Confirm event is emitted on initialization (line 68)
- [ ] **Verify PaymentReceived event**: Confirm event is emitted on first payment (line 122)
- [ ] **Verify MultiPaymentReceived event**: Confirm event is emitted on subsequent payments (line 124)
- [ ] **Verify SweepExecutedMulti event**: Confirm event is emitted on sweep/sweep_claim (lines 183, 235)
- [ ] **Verify AccountExpired event**: Confirm event is emitted on expire/recover (line 497)
- [ ] **Verify ReserveReclaimed event**: Confirm event is emitted on reserve reclamation (line 563-570)
- [ ] **Test event data integrity**: Verify all event fields contain accurate data

## Upgrade Path

- [ ] **Test upgrade mechanism**: Verify upgrade function works correctly with valid admin signature
- [ ] **Validate upgrade authorization**: Confirm only admin can upgrade (line 427-428)
- [ ] **Test upgrade state preservation**: Verify contract state is preserved across upgrades
- [ ] **Plan upgrade procedure**: Document the upgrade process for mainnet deployment

## Mainnet-Specific Configuration

- [ ] **Set appropriate expiry_ledger**: Confirm expiry_ledger values are appropriate for mainnet usage (consider ledger close times)
- [ ] **Validate network parameters**: Confirm all hardcoded values match mainnet network parameters
- [ ] **Test on testnet**: Deploy and test contract on public testnet before mainnet
- [ ] **Configure monitoring**: Set up monitoring for contract events and state changes

## Security Audit

- [ ] **Complete security audit**: Ensure contract has undergone professional security audit
- [ ] **Address audit findings**: Confirm all audit findings have been addressed or documented as accepted risks
- [ ] **Review threat model**: Verify threat model (docs/security.md) covers all identified risks
- [ ] **Test attack vectors**: Manually test common attack vectors (reentrancy, overflow, unauthorized access)

## Deployment Verification

- [ ] **Verify deployment script**: Confirm deployment scripts set all parameters correctly
- [ ] **Test deployment on testnet**: Successfully deploy to testnet with production-like configuration
- [ ] **Verify contract hash**: Confirm deployed contract hash matches compiled WASM
- [ ] **Validate initialization**: Test initialization with production parameters on testnet
- [ ] **Document deployment**: Create deployment runbook with step-by-step instructions

## Post-Deployment Monitoring

- [ ] **Set up event monitoring**: Configure monitoring for all contract events
- [ ] **Set up alerting**: Configure alerts for error conditions and unusual activity
- [ ] **Plan emergency response**: Document procedures for responding to security incidents
- [ ] **Prepare upgrade plan**: Have upgrade procedure ready in case of critical bugs
