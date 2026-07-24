# Sweep Authorization Signature Format

## Overview

The sweep controller uses **Ed25519 signature verification** to ensure only authorized parties can initiate sweeps. This document describes the exact message format that must be signed off-chain and provides implementation examples.

> **Correction:** an earlier version of this document included a `timestamp` component in the signed message, in every example below (TypeScript, Python, Rust) and in the Security Considerations and Troubleshooting sections. That was never accurate. The deployed contract — `contracts/sweep_controller/src/authorization.rs::construct_sweep_message()` — does not read, generate, or check a timestamp anywhere. It uses exactly **three** components. Every example in this revision has been corrected to match the real code; if you signed anything using the old examples, those signatures will not verify on-chain.

## Message Construction

> **Update (Issue #29):** the signed message now includes the **ephemeral
> account address** as its first component. This binds a signature to the
> specific account it authorizes, so a captured signature can no longer be
> replayed against a different ephemeral account served by the same controller.
> Signatures produced against the previous three-component format will no
> longer verify — off-chain signers must prepend the account XDR bytes.

The message to be signed is constructed as follows:

```
message = SHA256(
    account_address_xdr     ||
    destination_address_xdr ||
    sweep_nonce_be_u64      ||
    contract_id_xdr
)
```

### Components

1. **account_address_xdr** (variable length)
   - The ephemeral account the sweep authorizes funds to leave
   - Serialized as XDR bytes using `soroban_sdk::Address::to_xdr(&env)`, the same way as the destination
   - Binds the signature to a specific account — a signature valid for one ephemeral account will not verify for another

2. **destination_address_xdr** (variable length)
   - The wallet address where funds will be swept to
   - Serialized as XDR bytes using `soroban_sdk::Address::to_xdr(&env)` — the Soroban SDK's own serialization, not a hand-rolled encoding of the `G...`/`C...` strkey
   - Length varies by address type; don't assume a fixed size

3. **sweep_nonce** (8 bytes, big-endian)
   - Unsigned 64-bit integer
   - Starts at 0 for the first sweep (set at `initialize()`)
   - Increments by 1 after each successful sweep authorization
   - Prevents replay attacks by invalidating previous signatures
   - **The contract always verifies against its own current on-chain nonce.** Query it with `SweepController::get_nonce()` before signing — don't rely on a locally-tracked counter, which can drift if a sweep fails partway or another process triggers one.

4. **contract_id** (variable length)
   - The address of the sweep controller contract itself (`env.current_contract_address()`)
   - Serialized as XDR bytes the same way as the destination
   - Binds the signature to a specific contract deployment — a signature valid on one `SweepController` instance will not verify on another

There is no timestamp or expiry component. The concatenated bytes above are hashed exactly once with SHA-256, and that 32-byte digest is what gets Ed25519-signed.

### Hash Function

The concatenated message is hashed using **SHA-256**, producing a 32-byte digest that is then signed.

## Signature Scheme

- **Algorithm**: Ed25519
- **Public Key Size**: 32 bytes
- **Signature Size**: 64 bytes
- **Key Format**: Raw bytes (not PEM or other formats)

## Authorization Verification

The contract performs the following verification steps:

1. Retrieve the authorized signer public key from contract storage
2. Get the current sweep nonce and contract ID
3. Construct the message hash using the same algorithm as the off-chain signer
4. Verify the provided 64-byte signature against the message hash and public key — a failed verification traps the transaction rather than returning a recoverable error
5. If verification succeeds, increment the nonce to prevent replay

## Implementation Examples

> All three examples below construct `destination_xdr` / `contract_id_xdr` as opaque byte buffers you must supply — properly producing those bytes requires XDR-serializing a Soroban `Address` the same way `Address::to_xdr()` does on-chain. Hand-rolling that serialization is easy to get subtly wrong (wrong discriminant, wrong length prefix, etc.) and produces a signature that fails to verify with no useful error message. The canonical, verified way to get these bytes right is `tools/sweep-signer/` in this repo, which uses `soroban-sdk` itself to serialize the addresses — see [Reference Implementation](#reference-implementation) below. Treat the snippets here as illustrating the message-construction algorithm, not as production-ready XDR encoders.

### TypeScript Example

```typescript
import * as crypto from 'crypto';
import * as ed25519 from '@noble/ed25519';

interface SweepAuthParams {
  accountXdr: Buffer;       // ephemeral account Address::to_xdr() bytes — see note above
  destinationXdr: Buffer;   // Address::to_xdr() bytes — see note above
  contractIdXdr: Buffer;    // Address::to_xdr() bytes — see note above
  nonce: bigint;            // current on-chain nonce; query get_nonce() first
}

async function generateSweepSignature(
  params: SweepAuthParams,
  privateKey: Buffer
): Promise<Buffer> {
  // Convert nonce to big-endian bytes
  const nonceBuffer = Buffer.alloc(8);
  nonceBuffer.writeBigUInt64BE(params.nonce, 0);

  // Concatenate all components — destination, nonce, contract_id, in that order
  const message = Buffer.concat([
    params.accountXdr,
    params.destinationXdr,
    nonceBuffer,
    params.contractIdXdr,
  ]);

  // Hash the message with SHA-256
  const messageHash = crypto.createHash('sha256').update(message).digest();

  // Sign with Ed25519
  const signature = await ed25519.sign(messageHash, privateKey);

  return Buffer.from(signature);
}

// Verification example (for testing)
async function verifySweepSignature(
  params: SweepAuthParams,
  signature: Buffer,
  publicKey: Buffer
): Promise<boolean> {
  const nonceBuffer = Buffer.alloc(8);
  nonceBuffer.writeBigUInt64BE(params.nonce, 0);

  const message = Buffer.concat([
    params.accountXdr,
    params.destinationXdr,
    nonceBuffer,
    params.contractIdXdr,
  ]);

  const messageHash = crypto.createHash('sha256').update(message).digest();

  return await ed25519.verify(signature, messageHash, publicKey);
}

// Usage
const privateKeyHex = 'your-private-key-hex';
const privateKey = Buffer.from(privateKeyHex, 'hex');

const params: SweepAuthParams = {
  accountXdr: Buffer.from('...', 'base64'),     // properly XDR-encoded, see note above
  destinationXdr: Buffer.from('...', 'base64'), // properly XDR-encoded, see note above
  contractIdXdr: Buffer.from('...', 'base64'),  // properly XDR-encoded, see note above
  nonce: 0n,
};

const signature = await generateSweepSignature(params, privateKey);
console.log('Signature (hex):', signature.toString('hex'));
```

### Python Example

```python
import hashlib
import struct
from nacl.signing import SigningKey, VerifyKey
from nacl.exceptions import BadSignatureError

class SweepAuthSigner:
    def __init__(self, private_key_hex: str):
        """Initialize signer with Ed25519 private key."""
        self.private_key = SigningKey(bytes.fromhex(private_key_hex))
        self.verify_key = self.private_key.verify_key

    def construct_message(
        self,
        account_xdr: bytes,
        destination_xdr: bytes,
        contract_id_xdr: bytes,
        nonce: int,
    ) -> bytes:
        """Construct the message to be signed."""
        nonce_bytes = struct.pack('>Q', nonce)  # Big-endian unsigned 64-bit

        # Concatenate: account, destination, nonce, contract_id — no timestamp
        message = account_xdr + destination_xdr + nonce_bytes + contract_id_xdr

        return message

    def generate_signature(
        self,
        account_xdr: bytes,
        destination_xdr: bytes,
        contract_id_xdr: bytes,
        nonce: int,
    ) -> bytes:
        """Generate Ed25519 signature for sweep authorization."""
        message = self.construct_message(account_xdr, destination_xdr, contract_id_xdr, nonce)

        # Hash the message with SHA-256
        message_hash = hashlib.sha256(message).digest()

        # Sign with Ed25519
        signature = self.private_key.sign(message_hash).signature

        return signature

    def verify_signature(
        self,
        account_xdr: bytes,
        destination_xdr: bytes,
        contract_id_xdr: bytes,
        nonce: int,
        signature: bytes,
    ) -> bool:
        """Verify sweep authorization signature."""
        message = self.construct_message(account_xdr, destination_xdr, contract_id_xdr, nonce)
        message_hash = hashlib.sha256(message).digest()

        try:
            self.verify_key.verify(message_hash, signature)
            return True
        except BadSignatureError:
            return False


# Usage
private_key_hex = 'your-private-key-hex'
signer = SweepAuthSigner(private_key_hex)

account_xdr = b'...'  # XDR-encoded ephemeral account address, see note above
destination_xdr = b'...'  # XDR-encoded destination address, see note above
contract_id_xdr = b'...'  # XDR-encoded contract ID, see note above
nonce = 0  # query SweepController.get_nonce() first — don't hardcode in real use

signature = signer.generate_signature(account_xdr, destination_xdr, contract_id_xdr, nonce)
print('Signature (hex):', signature.hex())

# Verify
is_valid = signer.verify_signature(account_xdr, destination_xdr, contract_id_xdr, nonce, signature)
print(f'Signature valid: {is_valid}')
```

### Rust Example (Off-chain)

```rust
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use sha2::{Sha256, Digest};

pub struct SweepAuthSigner {
    signing_key: SigningKey,
}

impl SweepAuthSigner {
    pub fn new(private_key_bytes: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(private_key_bytes);
        Self { signing_key }
    }

    pub fn construct_message(
        account_xdr: &[u8],
        destination_xdr: &[u8],
        contract_id_xdr: &[u8],
        nonce: u64,
    ) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(account_xdr);
        message.extend_from_slice(destination_xdr);
        message.extend_from_slice(&nonce.to_be_bytes());
        message.extend_from_slice(contract_id_xdr);
        message
    }

    pub fn generate_signature(
        &self,
        account_xdr: &[u8],
        destination_xdr: &[u8],
        contract_id_xdr: &[u8],
        nonce: u64,
    ) -> Vec<u8> {
        let message = Self::construct_message(account_xdr, destination_xdr, contract_id_xdr, nonce);

        let mut hasher = Sha256::new();
        hasher.update(&message);
        let message_hash = hasher.finalize();

        let signature = self.signing_key.sign(&message_hash);
        signature.to_bytes().to_vec()
    }

    pub fn verify_signature(
        &self,
        account_xdr: &[u8],
        destination_xdr: &[u8],
        contract_id_xdr: &[u8],
        nonce: u64,
        signature_bytes: &[u8; 64],
    ) -> bool {
        let message = Self::construct_message(account_xdr, destination_xdr, contract_id_xdr, nonce);

        let mut hasher = Sha256::new();
        hasher.update(&message);
        let message_hash = hasher.finalize();

        let verify_key = VerifyingKey::from(&self.signing_key);
        verify_key.verify_strict(&message_hash, signature_bytes).is_ok()
    }
}

// Usage
let private_key_bytes = [0u8; 32]; // Load from secure storage
let signer = SweepAuthSigner::new(&private_key_bytes);

let account_xdr = b"..."; // XDR-encoded ephemeral account, see note above
let destination_xdr = b"..."; // XDR-encoded destination, see note above
let contract_id_xdr = b"..."; // XDR-encoded contract ID, see note above
let nonce = 0u64; // query get_nonce() first — don't hardcode in real use

let signature = signer.generate_signature(account_xdr, destination_xdr, contract_id_xdr, nonce);
println!("Signature: {}", hex::encode(&signature));
```

### Reference Implementation

Rather than any of the illustrative snippets above, the tool actually checked against the real `soroban-sdk` XDR serialization lives at `tools/sweep-signer/` in this repo. It's a small Rust CLI that:
- Takes a Stellar secret key, destination address, contract ID, and nonce
- Uses `soroban_sdk::Address::to_xdr()` directly (via a local, network-free `Env`) to guarantee byte-identical serialization to what the deployed contract computes
- Outputs the hex signature ready to pass to `execute_sweep()`

See its `--help` output or the repo README for usage. If you're building an off-chain signing service in another language, the safest path today is to shell out to this tool (or a compiled build of it) rather than re-deriving the XDR bytes independently.

## Integration with Off-Chain System

The off-chain system should:

1. **Receive sweep request** from the user with destination address and amount
2. **Query current contract state** to get:
   - Current nonce, via `SweepController::get_nonce()`
   - Contract ID (the deployed `SweepController` address)
3. **Construct message** using the format above (account, destination, nonce, contract_id — no timestamp)
4. **Sign message** with the authorized signer's private key
5. **Call `execute_sweep` contract function** with the generated signature

## Security Considerations

### Replay Attack Prevention

- The **nonce mechanism** ensures each sweep signature is unique
- After successful authorization, the nonce is incremented
- Attempting to reuse an old signature will fail because the nonce has changed
- There is currently no way to query the nonce other than calling `get_nonce()` directly on the deployed contract — don't assume a value without checking

### Signature Validity

- Signatures are **bound to a specific ephemeral account** via account_address — a signature authorizing one account cannot be replayed against another account served by the same controller
- Signatures are **bound to a specific contract deployment** via contract_id
- Signatures cannot be used against a different deployment
- Signatures do **not** expire based on time — there is no timestamp or expiry window in this scheme. The only thing that invalidates a previously-issued, not-yet-used signature is the nonce advancing (i.e. another sweep happening first). If you need time-bounded authorization, that would have to be built as a new feature — it does not exist today.

### Key Management

- **Private keys** must be stored securely (HSM, encrypted storage, key management service)
- **Public keys** are stored on-chain and can be rotated via contract initialization
- Never expose private keys in logs or error messages

## Troubleshooting

### "AuthorizedSignerNotSet" Error
- The sweep controller has not been initialized
- Call `initialize()` with the authorized signer public key first

### "SignatureVerificationFailed" Error
- The signature does not match the expected message
- Verify that all message components are constructed correctly, in order: account XDR, then destination XDR, then 8-byte big-endian nonce, then contract ID XDR — no timestamp
- Ensure the correct public key is being used for verification
- Check that the nonce used matches the contract's current `get_nonce()` value at the moment of signing — it may have advanced since you last checked

### "InvalidSignature" Error
- The signature format is incorrect (must be exactly 64 bytes)
- The signature was corrupted during transmission
- The message was modified after signing

## Testing

To generate a standalone Ed25519 keypair for testing signature verification against a locally-deployed contract:

```bash
# Generate Ed25519 keypair
openssl genpkey -algorithm ed25519 -out private.pem
openssl pkey -in private.pem -pubout -out public.pem

# Extract raw public key (32 bytes)
openssl pkey -outform DER -pubout -in private.pem | tail -c 32 | xxd -p

# Extract raw private key (32 bytes)
openssl pkey -outform DER -in private.pem | tail -c 32 | xxd -p
```

Note: this gives you a generic raw Ed25519 keypair, not a Stellar-strkey-formatted one — fine for setting `authorized_signer` directly as raw bytes at `initialize()`, but if you need an `S...`/`G...` Stellar keypair instead (e.g. to reuse `tools/sweep-signer`, which accepts an `S...` secret), generate it with `stellar keys generate` instead — see the root README's "Getting testnet keys" guidance.

Then use the examples above, or `tools/sweep-signer`, to generate and verify test signatures.