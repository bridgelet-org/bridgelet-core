# Runbook: Rotating `ReserveContract`'s Admin Key

**Audience:** operators / SREs / on-call engineers responsible for the Bridgelet Core deployment.

**Scope:** rotating the admin address stored inside a deployed `ReserveContract` instance (`contracts/reserve_contract/src/lib.rs`).

**When to use this runbook:** the admin secret key has been (or is suspected of being) compromised; an operator is leaving the team; periodic scheduled rotation; any time the current admin address should no longer hold write authority over `set_base_reserve`.

---

## Important: there is no in-place `transfer_admin`

`ReserveContract` (see [`contracts/reserve_contract/src/lib.rs`](../../contracts/reserve_contract/src/lib.rs)) currently exposes exactly one admin-writing function: `initialize(env, admin)`. The `DataKey::Admin` slot is written once at `initialize()` and never mutated, because the contract has **no** `transfer_admin` (or equivalent) function today. Verifying this against the source:

```rust
// DataKey::Admin is only ever written in storage::set_admin,
// and set_admin is only called from ReserveContract::initialize.
// No other public function writes to DataKey::Admin.
```

That means there is no path that lets you rotate the admin of an already-deployed instance without redeployment. Any runbook that claims otherwise — e.g. "just call `transfer_admin(new_admin)`" — is incorrect for the current contract code and must not be followed.

The remainder of this document gives the only rotation procedures that are compatible with the deployed contract as it is committed today.

---

## Procedure A — Standard rotation via redeploy + re-initialize (recommended)

Use this when the goal is to keep the contract instance address identical for downstream consumers.

### Preconditions

- You control the **deployer** key for the network where `ReserveContract` is live (the same key that originally deployed the contract). This is the only auth identity that can install and deploy a new `ReserveContract` WASM.
- You have access to both the **old** admin secret key and the **new** admin address (`G…` or `C…` strkey format).
- The new admin address has been verified out-of-band (key ceremony, hardware wallet confirmation, etc.).
- No live users / SDKs are reading `get_admin()` against this exact contract instance; if any are, schedule a maintenance window — see [Caveats](#caveats) below.

### Step-by-step

1. **Pre-flight read — capture the current admin.**

   ```bash
   stellar contract invoke \
       --id "$RESERVE_CONTRACT_ID" \
       --network "$NETWORK" \
       --rpc-url "$SOROBAN_RPC_URL" \
       --network-passphrase "$NETWORK_PASSPHRASE" \
       -- get_admin
   ```

   Confirm the returned address matches the old admin you intend to rotate out. Abort the procedure if it doesn't — that means you are about to redeploy against the wrong instance.

2. **Pre-flight read — capture the current configured reserve value.**

   ```bash
   stellar contract invoke \
       --id "$RESERVE_CONTRACT_ID" \
       --network "$NETWORK" \
       --rpc-url "$SOROBAN_RPC_URL" \
       --network-passphrase "$NETWORK_PASSPHRASE" \
       -- get_base_reserve
   ```

   Save this value. The redeployed contract will start with no configured reserve; you will need to re-apply the save value using the **new** admin key.

3. **Build the WASM** (no source changes are required to do a rotation — you are rebuilding the exact same artifact):

   ```bash
   ./scripts/build.sh
   ```

   Confirm `target/wasm32v1-none/release/reserve_contract.wasm` exists.

4. **Deploy the new instance.** Use the same deployer key as the original deployment:

   ```bash
   NEW_RESERVE_CONTRACT_ID=$(stellar contract deploy \
       --wasm target/wasm32v1-none/release/reserve_contract.wasm \
       --source "$SIGNER_SECRET_KEY" \
       --network "$NETWORK" \
       --rpc-url "$SOROBAN_RPC_URL" \
       --network-passphrase "$NETWORK_PASSPHRASE")
   echo "New ReserveContract instance: $NEW_RESERVE_CONTRACT_ID"
   ```

5. **Re-initialize with the new admin.** This call must be authorized by the **new** admin address (Soroban native auth):

   ```bash
   stellar contract invoke \
       --id "$NEW_RESERVE_CONTRACT_ID" \
       --source "$NEW_ADMIN_SECRET_KEY" \
       --network "$NETWORK" \
       --rpc-url "$SOROBAN_RPC_URL" \
       --network-passphrase "$NETWORK_PASSPHRASE" \
       -- initialize \
       --admin "$NEW_ADMIN_ADDRESS"
   ```

   Confirm a `ContractInitialized { admin: <NEW_ADMIN> }` event appears on the new instance.

6. **Re-apply the stored reserve value.** Authorize as the **new** admin:

   ```bash
   stellar contract invoke \
       --id "$NEW_RESERVE_CONTRACT_ID" \
       --source "$NEW_ADMIN_SECRET_KEY" \
       --network "$NETWORK" \
       --rpc-url "$SOROBAN_RPC_URL" \
       --network-passphrase "$NETWORK_PASSPHRASE" \
       -- set_base_reserve \
       --amount "$SAVED_RESERVE_VALUE_STROOPS"
   ```

   The contract will reject `amount <= 0` (`Error::InvalidAmount`) and `amount > 100_000_000_000` (`Error::AmountTooLarge`). The saved value was previously valid, so these should not trip; if they do, stop and investigate before re-applying.

7. **Verify post-rotation.** This is the verification step required by the originating issue:

   ```bash
   # Admin must reflect the new address
   stellar contract invoke \
       --id "$NEW_RESERVE_CONTRACT_ID" \
       --network "$NETWORK" --rpc-url "$SOROBAN_RPC_URL" \
       --network-passphrase "$NETWORK_PASSPHRASE" \
       -- get_admin
   # Expected: <NEW_ADMIN_ADDRESS>

   # Reserve value must reflect the saved value
   stellar contract invoke \
       --id "$NEW_RESERVE_CONTRACT_ID" \
       --network "$NETWORK" --rpc-url "$SOROBAN_RPC_URL" \
       --network-passphrase "$NETWORK_PASSPHRASE" \
       --get_base_reserve
   # Expected: Some(SAVED_RESERVE_VALUE_STROOPS)
   ```

   If `get_admin()` does not return the expected new address, treat the rotation as **failed** — do not proceed to step 8 — and consult [`../faq/why-salt-based-deployment.md`](../faq/why-salt-based-deployment.md) (when available) and the broader sweep / reserve integration notes in [`docs/architecture.md`](../../docs/architecture.md).

8. **Coordinate the cutover.** Distribute the new contract ID:

   - Update `deployments/<network>.json` and any CI secret (`RESERVE_CONTRACT_ID` in `scripts/deploy-testnet.sh`).
   - Notify SDK integrators and any operators reading `get_admin()` from this instance to switch their pinned ID.
   - Mark the old instance ID as deprecated in runbooks; **do not delete** the old instance — its TTL is extended on every read (`storage::extend_instance_ttl`), so it will persist for some time regardless.

9. **Retire the old admin key.** Once all callers have cut over, treat the old admin secret as rotated-out:

   - Revoke any HSM / KMS access for the old admin key.
   - If the rotation was triggered by a compromise, follow [`security-disclosure-triage.md`](./security-disclosure-triage.md) and the post-rotation hardening steps there.

### Caveats

- **Reference compatibility.** Other Bridgelet contracts (`EphemeralAccount`, `SweepController`, `AccountFactory`) do **not** read `ReserveContract::get_admin()` or hold a reference to its address. Cross-reference [`docs/architecture.md`](../../docs/architecture.md) for the current integration status; as of the latest revision, `ReserveContract` is purely a standalone config contract, so redeploying does not require coordinated updates to the other contracts.
- **State mismatch window.** Between `deploy` and `set_base_reserve`, consumers calling `get_base_reserve()` on the new instance will see `None`, and consumers calling `require_base_reserve()` will see `Error::ReserveNotSet`. Keep this window as small as possible.
- **Storage migration.** Today there is no on-chain link between the old and new instances. If your off-chain indexer keys events by contract ID, plan to re-index against the new ID.

---

## Procedure B — Add a `transfer_admin` function (long-term fix)

This is the **code-change** path. It is **not** what you should do during an emergency rotation — go to [Procedure A](#procedure-a--standard-rotation-via-redeploy--re-initialize-recommended). Use Procedure B to permanently close the gap that forces redeployment today.

Plan:

1. Add a new function to `ReserveContract`:

   ```rust
   pub fn transfer_admin(env: Env, new_admin: Address) -> Result<(), Error> {
       storage::extend_instance_ttl(&env);
       let admin = storage::get_admin(&env).ok_or(Error::NotInitialized)?;
       admin.require_auth();

       let old_admin = admin.clone();
       storage::set_admin(&env, &new_admin);
       events::emit_admin_transferred(&env, old_admin, new_admin);
       Ok(())
   }
   ```

2. Add a new error variant `TransferAdminFailed` (or similar) to [`contracts/reserve_contract/src/errors.rs`](../../contracts/reserve_contract/src/errors.rs) if needed.

3. Add a new event variant `AdminTransferred { old_admin, new_admin }` in [`contracts/reserve_contract/src/events.rs`](../../contracts/reserve_contract/src/events.rs).

4. Add a unit test in [`contracts/reserve_contract/src/test.rs`](../../contracts/reserve_contract/src/test.rs): only the current admin can call `transfer_admin`; `get_admin()` reflects the new address afterward; the previous admin can no longer write.

5. Use `EphemeralAccount`'s own [`upgrade()`](../../contracts/ephemeral_account/src/lib.rs) flow (or a fresh deploy, depending on the production upgrade strategy) to roll the new WASM. See [./upgrade-admin-authority.md](./upgrade-admin-authority.md) for the broader context.

Once Procedure B ships, [Procedure A](#procedure-a--standard-rotation-via-redeploy--re-initialize-recommended) becomes the fallback / discoverability-of-state path, and the emergency procedure becomes a single `transfer_admin` call.

---

## Cross-references

- [`upgrade-admin-authority.md`](./upgrade-admin-authority.md) — broader authority-rotation context (contract WASM upgrades, including the `upgrade()` flow on `EphemeralAccount`).
- [`security-disclosure-triage.md`](./security-disclosure-triage.md) — what to do when the rotation is triggered by a suspected compromise rather than a scheduled event.
- [`docs/architecture.md`](../../docs/architecture.md) — current integration status of `ReserveContract` with the rest of the system (it is currently a standalone config contract; nothing else on-chain reads from it).
- [`contracts/reserve_contract/src/lib.rs`](../../contracts/reserve_contract/src/lib.rs) — the contract source. Re-read before any code-level changes.
