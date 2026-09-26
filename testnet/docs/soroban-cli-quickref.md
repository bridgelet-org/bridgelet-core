# Soroban CLI Quick Reference

Essential command-line snippets for interacting with Bridgelet Core contracts on Stellar Testnet using `stellar-cli` / `soroban-cli`.

## Useful Commands

```bash
# Identity & Account Management
stellar keys generate alice --network testnet
stellar keys fund alice --network testnet

# Contract Invocation & Simulation
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source alice \
  --network testnet \
  -- \
  can_sweep --account <EPHEMERAL_ADDRESS>
```

## Pre-flight Simulation Best Practices

Always run transaction simulation using `simulateTransaction` prior to signing and submitting state-changing calls to avoid unnecessary transaction fee failures.
