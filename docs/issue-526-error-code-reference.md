# Contract Error Code Reference

Closes #526. Consolidates every `#[contracterror] enum Error` across `contracts/*/src/errors.rs` as it exists in source today (verified by grepping discriminants directly, not hand-maintained).

## Namespace scheme (Issue #248)

`ephemeral_account/src/errors.rs` defines a 1000-wide-block convention so a raw `u32` code can be traced to its contract without the originating address. Adoption is **inconsistent**:

| Contract | Claimed range | Actual codes in use |
|---|---|---|
| ephemeral_account | 1000-1999 | 1000-1015 (16 variants) |
| sweep_controller | 2000-2999 | 2000-2018 (18 variants, 2011 gapped) |
| reserve_contract | 3000-3999 | 3000-3005 (7 variants) |
| timelock_controller | 4000-4099 | 4000-4008 (9 variants) |
| multisig_approval | 5000-5099 | 5000-5008 (9 variants) |
| fee_splitter | 6000-6099 | 6000-6007 (8 variants) |
| **nonce_registry** | **7000-7099** | **7000** (1 variant) |
| **fee_sponsor** | **7000-7999** | **7000-7009** (10 variants) |
| allowlist_registry | 8000-8099 | 8000-8002 (3 variants) |
| **asset_allowlist** | **9000-9099** | **9000-9002** (3 variants) |
| **version_registry** | **9000-9099** | **9000-9003** (4 variants) |
| audit_log | 9100-9199 | 9100-9104 (5 variants) |
| access_controller | (undocumented) | 10000-10003 (4 variants) |
| notification_registry | 11000-11099 | 11000 (1 variant) |
| expiry_scheduler | (undocumented) | 12000-12003 (4 variants) |
| metrics_aggregator | (undocumented) | 13000-13005 (6 variants) |

## Finding: confirmed range collisions

Two pairs claim (and use) the same numeric block, so a bare error code can't be resolved to one contract: `nonce_registry::NonceAlreadyConsumed = 7000` collides with `fee_sponsor::AlreadyInitialized = 7000`; and `asset_allowlist`'s `{AlreadyInitialized=9000, NotInitialized=9001, ...}` collides variant-for-variant with `version_registry`'s identical sequence. Both pairs' header comments claim conflicting ownership of the same block.

## Contracts outside the scheme entirely

`account_factory`, `batch_sweep_queue`, `claimable_balance_registry`, `compliance_oracle`, `escrow_vault`, `pause_guardian`, `rate_limiter` use small sequential codes (1, 2, 3, ...) with no per-contract block — never migrated under Issue #248, and guaranteed to collide with each other and with every namespaced contract's low end.

## Recommendation

Resolve the two active collisions, migrate the un-namespaced contracts onto reserved blocks (14000+), and add a CI check (companion to #529) that fails the build if two `errors.rs` files declare overlapping discriminant ranges.
