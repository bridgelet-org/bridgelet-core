# Signed Message Construction (SHA-256 over destination + nonce + contract_id)

## Purpose
This document specifies the exact byte layout that `construct_sweep_message()` builds before hashing. It serves as the canonical reference that any off-chain signer implementation should be built against.

## Byte Layout

The sweep message is constructed by concatenating three specific components in the following exact order:

1.  **Destination**: The XDR-serialized destination address.
2.  **Nonce**: The nonce represented as a big-endian 64-bit unsigned integer (`u64`).
3.  **Contract ID**: The XDR-serialized `contract_id`.

## Domain Separation

The `contract_id` is included as the final component of the message construction to provide domain separation. By including the specific contract ID, a signature generated for one deployment of the `SweepController` cannot be replayed on a different deployment, even if they happen to share the same nonce and target the same destination. This ensures that signatures are strictly bound to a single, specific smart contract instance.
