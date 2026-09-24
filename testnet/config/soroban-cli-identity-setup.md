# Configuring a Local Soroban CLI Identity for Testnet

This guide walks through creating and funding a local `soroban-cli`/`stellar-cli` identity specifically pointed at **Stellar Testnet**, so you can interact with Bridgelet testnet contracts without inferring commands from deploy scripts.

---

## Prerequisites

- [stellar-cli](https://github.com/stellar/stellar-cli) installed (`cargo install --locked stellar-cli --version 23.4.1`)
- Network access to Stellar Testnet (Horizon: `https://horizon-testnet.stellar.org`, RPC: `https://soroban-testnet.stellar.org`)

---

## 1. Generate a New Keypair

```bash
# Generate a new random keypair
stellar keys generate --global testnet-identity

# Verify it was created
stellar keys ls
```

**Output example:**
```
testnet-identity (global)  GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

The public key (starting with `G...`) is your **account ID**. The secret key is stored encrypted in the global key store.

---

## 2. Fund the Account on Testnet

Testnet accounts must be funded via Friendbot before they can submit transactions.

```bash
# Fund using Friendbot (requires the public key)
stellar keys fund testnet-identity --network testnet
```

**Alternative (manual):**
```bash
# Get the public key
PUBKEY=$(stellar keys address testnet-identity)

# Fund via curl to Friendbot
curl "https://friendbot.stellar.org?addr=$PUBKEY"
```

**Verify funding:**
```bash
stellar keys balance testnet-identity --network testnet
```

---

## 3. Add Testnet Network Configuration

Ensure the `testnet` network is configured in your stellar-cli:

```bash
# Add testnet network (if not already present)
stellar network add --global testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"
```

**Verify network config:**
```bash
stellar network ls
```

---

## 4. Set Default Network and Identity (Optional)

```bash
# Set testnet as default network
stellar config set --global network testnet

# Set testnet-identity as default identity
stellar config set --global identity testnet-identity
```

With these defaults, you can omit `--network testnet --source testnet-identity` from subsequent commands.

---

## 5. Verify End-to-End

Test that you can invoke a read-only contract method on testnet:

```bash
# Example: Check if an EphemeralAccount is expired
# Replace <EPHEMERAL_CONTRACT_ID> with a real testnet contract ID
stellar contract invoke \
  --id <EPHEMERAL_CONTRACT_ID> \
  --network testnet \
  --source testnet-identity \
  -- \
  is_expired
```

---

## 6. Using with Bridgelet Testnet Contracts

### Initialize an EphemeralAccount

```bash
stellar contract invoke \
  --id <EPHEMERAL_ACCOUNT_WASM_HASH> \
  --network testnet \
  --source testnet-identity \
  -- \
  initialize \
  --creator <CREATOR_ADDRESS> \
  --expiry_ledger <FUTURE_LEDGER_SEQUENCE> \
  --recovery_address <RECOVERY_ADDRESS> \
  --authorized_controller <SWEEP_CONTROLLER_CONTRACT_ID> \
  --admin <ADMIN_ADDRESS>
```

### Initialize SweepController

```bash
stellar contract invoke \
  --id <SWEEP_CONTROLLER_WASM_HASH> \
  --network testnet \
  --source testnet-identity \
  -- \
  initialize \
  --creator <CREATOR_ADDRESS> \
  --authorized_signer <ED25519_PUBLIC_KEY_32_BYTES_HEX> \
  --authorized_destination <DESTINATION_ADDRESS>
```

### Record a Payment

```bash
stellar contract invoke \
  --id <EPHEMERAL_CONTRACT_ID> \
  --network testnet \
  --source testnet-identity \
  -- \
  record_payment \
  --amount 100000000 \
  --asset <TOKEN_CONTRACT_ID>
```

### Execute Sweep (via SweepController)

```bash
stellar contract invoke \
  --id <SWEEP_CONTROLLER_CONTRACT_ID> \
  --network testnet \
  --source testnet-identity \
  -- \
  execute_sweep \
  --ephemeral_account <EPHEMERAL_CONTRACT_ID> \
  --destination <DESTINATION_ADDRESS> \
  --auth_signature <64_BYTE_ED25519_SIG_HEX>
```

---

## 7. Managing Multiple Identities

For different roles (deployer, relayer, recipient), create separate identities:

```bash
# Deployer identity (admin keys)
stellar keys generate --global testnet-deployer
stellar keys fund testnet-deployer --network testnet

# Relayer identity (pays fees for claim())
stellar keys generate --global testnet-relayer
stellar keys fund testnet-relayer --network testnet

# Recipient identity (signs claim auth)
stellar keys generate --global testnet-recipient
stellar keys fund testnet-recipient --network testnet
```

---

## 8. Troubleshooting

| Issue | Resolution |
|-------|------------|
| `Error: account not found` | Account not funded. Run `stellar keys fund <identity> --network testnet` |
| `Error: failed to connect to RPC` | Check RPC URL: `stellar network ls` and verify `soroban-testnet.stellar.org` is reachable |
| `Error: invalid network passphrase` | Ensure network passphrase is exactly `Test SDF Network ; September 2015` |
| `Error: insufficient balance` | Fund the identity with more XLM via Friendbot |

---

## 9. Security Notes

- **Never commit secret keys** to version control. The global key store (`~/.config/stellar/keys/`) is local only.
- Use **separate identities** for deployer/admin vs. relayer vs. recipient to follow least-privilege.
- Rotate testnet identities periodically; Friendbot rate limits apply per IP.

---

## 10. Reference

- [stellar-cli documentation](https://github.com/stellar/stellar-cli)
- [Soroban Testnet RPC](https://soroban-testnet.stellar.org)
- [Friendbot](https://friendbot.stellar.org)
- Bridgelet testnet contract IDs: see `testnet/registry/contract-status.md`