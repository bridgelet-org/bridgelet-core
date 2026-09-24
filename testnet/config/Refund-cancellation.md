Refund/cancellation path for ephemeral account creation that fails mid-transaction
Repo Avatar
bridgelet-org/bridgelet-core
Location
contracts/ephemeral_account/src/lib.rs

Problem
If account creation involves multiple steps (e.g. account creation followed by funding, followed by trustline setup once the trustline-management issue lands) and a later step fails, no confirmed path exists to safely unwind or refund the funding account for the portion that did complete, risking silently stranded funds in a half-created account.

Acceptance Criteria
 Multi-step creation flow reviewed for exactly which failure points leave funds stranded versus safely atomic
 A refund/recovery path implemented for any identified stranded-funds scenario, or the flow restructured to be atomic where feasible
 Test covers at least one induced mid-flow failure and confirms funds are recoverable, not stuck

