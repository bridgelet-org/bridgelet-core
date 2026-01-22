# Sweep Authorization Signature Format

## Overview

The sweep controller uses **Ed25519 signature verification** to ensure only authorized parties can initiate sweeps. This document describes the exact message format that must be signed off-chain and provides implementation examples.

## Message Construction

The message to be signed is constructed as follows:

```
message = SHA256(
    destination_address ||
    sweep_nonce ||
    contract_id ||
    timestamp
)
```

### Components

1. **destination_address** (variable length)
   - The wallet address where funds will be swept to
   - Serialized as XDR bytes (Soroban Address format)
   - Approximately 32-40 bytes depending on account type

2. **sweep_nonce** (8 bytes, big-endian)
   - Unsigned 64-bit integer
   - Starts at 0 for the first sweep
   - Increments by 1 after each successful authorization
   - Prevents replay attacks by invalidating previous signatures

3. **contract_id** (variable length)
   - The address of the sweep controller contract itself
   - Serialized as XDR bytes (Soroban Address format)
   - Binds the signature to a specific contract deployment

4. **timestamp** (8 bytes, big-endian)
   - Current Unix timestamp in seconds
   - Retrieved from the Soroban ledger at verification time
   - Prevents accidental use of stale signatures across time

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
2. Get the current sweep nonce, contract ID, and current timestamp
3. Construct the message hash using the same algorithm as the off-chain signer
4. Verify the provided 64-byte signature against the message hash and public key
5. If verification succeeds, increment the nonce to prevent replay

## Implementation Examples

### TypeScript Example

```typescript
import * as crypto from 'crypto';
import * as ed25519 from '@noble/ed25519';

interface SweepAuthParams {
  destination: string;        // Soroban address
  contractId: string;         // Soroban address
  nonce: bigint;             // Current nonce
  timestamp: bigint;         // Current Unix timestamp
}

async function generateSweepSignature(
  params: SweepAuthParams,
  privateKey: Buffer
): Promise<Buffer> {
  // Convert addresses to XDR bytes (simplified - actual implementation uses soroban-js)
  const destinationXdr = Buffer.from(params.destination, 'base64'); // Properly XDR-encoded
  const contractXdrId = Buffer.from(params.contractId, 'base64');   // Properly XDR-encoded

  // Convert nonce to big-endian bytes
  const nonceBuffer = Buffer.alloc(8);
  nonceBuffer.writeBigUInt64BE(params.nonce, 0);

  // Convert timestamp to big-endian bytes
  const timestampBuffer = Buffer.alloc(8);
  timestampBuffer.writeBigUInt64BE(params.timestamp, 0);

  // Concatenate all components
  const message = Buffer.concat([
    destinationXdr,
    nonceBuffer,
    contractXdrId,
    timestampBuffer,
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
  // Same message construction
  const destinationXdr = Buffer.from(params.destination, 'base64');
  const contractXdrId = Buffer.from(params.contractId, 'base64');

  const nonceBuffer = Buffer.alloc(8);
  nonceBuffer.writeBigUInt64BE(params.nonce, 0);

  const timestampBuffer = Buffer.alloc(8);
  timestampBuffer.writeBigUInt64BE(params.timestamp, 0);

  const message = Buffer.concat([
    destinationXdr,
    nonceBuffer,
    contractXdrId,
    timestampBuffer,
  ]);

  const messageHash = crypto.createHash('sha256').update(message).digest();

  // Verify with Ed25519
  return await ed25519.verify(signature, messageHash, publicKey);
}

// Usage
const privateKeyHex = 'your-private-key-hex';
const privateKey = Buffer.from(privateKeyHex, 'hex');

const params: SweepAuthParams = {
  destination: 'GBRPYHIL2CI3...', // Soroban address
  contractId: 'CBVG...', // Sweep controller contract ID
  nonce: 0n,
  timestamp: BigInt(Math.floor(Date.now() / 1000)),
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
        destination_xdr: bytes,
        contract_id_xdr: bytes,
        nonce: int,
        timestamp: int,
    ) -> bytes:
        """Construct the message to be signed."""
        # Convert nonce and timestamp to big-endian bytes
        nonce_bytes = struct.pack('>Q', nonce)  # Big-endian unsigned 64-bit
        timestamp_bytes = struct.pack('>Q', timestamp)  # Big-endian unsigned 64-bit

        # Concatenate all components
        message = (
            destination_xdr +
            nonce_bytes +
            contract_id_xdr +
            timestamp_bytes
        )

        return message

    def generate_signature(
        self,
        destination_xdr: bytes,
        contract_id_xdr: bytes,
        nonce: int,
        timestamp: int,
    ) -> bytes:
        """Generate Ed25519 signature for sweep authorization."""
        message = self.construct_message(
            destination_xdr,
            contract_id_xdr,
            nonce,
            timestamp,
        )

        # Hash the message with SHA-256
        message_hash = hashlib.sha256(message).digest()

        # Sign with Ed25519
        signature = self.private_key.sign(message_hash).signature

        return signature

    def verify_signature(
        self,
        destination_xdr: bytes,
        contract_id_xdr: bytes,
        nonce: int,
        timestamp: int,
        signature: bytes,
    ) -> bool:
        """Verify sweep authorization signature."""
        message = self.construct_message(
            destination_xdr,
            contract_id_xdr,
            nonce,
            timestamp,
        )

        message_hash = hashlib.sha256(message).digest()

        try:
            self.verify_key.verify(message_hash, signature)
            return True
        except BadSignatureError:
            return False


# Usage
private_key_hex = 'your-private-key-hex'
signer = SweepAuthSigner(private_key_hex)

destination_xdr = b'...'  # XDR-encoded destination address
contract_id_xdr = b'...'  # XDR-encoded contract ID
nonce = 0
timestamp = int(time.time())

signature = signer.generate_signature(
    destination_xdr,
    contract_id_xdr,
    nonce,
    timestamp,
)

print('Signature (hex):', signature.hex())

# Verify
is_valid = signer.verify_signature(
    destination_xdr,
    contract_id_xdr,
    nonce,
    timestamp,
    signature,
)
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
        destination_xdr: &[u8],
        contract_id_xdr: &[u8],
        nonce: u64,
        timestamp: u64,
    ) -> Vec<u8> {
        let mut message = Vec::new();

        // Add destination XDR bytes
        message.extend_from_slice(destination_xdr);

        // Add nonce as big-endian bytes
        message.extend_from_slice(&nonce.to_be_bytes());

        // Add contract ID XDR bytes
        message.extend_from_slice(contract_id_xdr);

        // Add timestamp as big-endian bytes
        message.extend_from_slice(&timestamp.to_be_bytes());

        message
    }

    pub fn generate_signature(
        &self,
        destination_xdr: &[u8],
        contract_id_xdr: &[u8],
        nonce: u64,
        timestamp: u64,
    ) -> Vec<u8> {
        let message = Self::construct_message(
            destination_xdr,
            contract_id_xdr,
            nonce,
            timestamp,
        );

        // Hash with SHA-256
        let mut hasher = Sha256::new();
        hasher.update(&message);
        let message_hash = hasher.finalize();

        // Sign with Ed25519
        let signature = self.signing_key.sign(&message_hash);
        signature.to_bytes().to_vec()
    }

    pub fn verify_signature(
        &self,
        destination_xdr: &[u8],
        contract_id_xdr: &[u8],
        nonce: u64,
        timestamp: u64,
        signature_bytes: &[u8; 64],
    ) -> bool {
        let message = Self::construct_message(
            destination_xdr,
            contract_id_xdr,
            nonce,
            timestamp,
        );

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

let destination_xdr = b"..."; // XDR-encoded destination
let contract_id_xdr = b"..."; // XDR-encoded contract ID
let nonce = 0u64;
let timestamp = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_secs();

let signature = signer.generate_signature(
    destination_xdr,
    contract_id_xdr,
    nonce,
    timestamp,
);

println!("Signature: {}", hex::encode(&signature));
```

## Integration with Off-Chain System

The off-chain system should:

1. **Receive sweep request** from the user with destination address and amount
2. **Query current contract state** to get:
   - Current nonce
   - Contract ID
   - Current timestamp
3. **Construct message** using the format above
4. **Sign message** with the authorized signer's private key
5. **Call `execute_sweep` contract function** with the generated signature

## Security Considerations

### Replay Attack Prevention

- The **nonce mechanism** ensures each sweep signature is unique
- After successful authorization, the nonce is incremented
- Attempting to reuse an old signature will fail because the nonce has changed

### Signature Validity

- Signatures are **bound to a specific contract deployment** via contract_id
- Signatures cannot be used against a different deployment
- The **timestamp component** allows for potential future time-based restrictions

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
- Verify that all message components are constructed correctly
- Ensure the correct public key is being used for verification
- Check that nonce values are synchronized (they increment after each successful sweep)

### "InvalidSignature" Error
- The signature format is incorrect (must be exactly 64 bytes)
- The signature was corrupted during transmission
- The message was modified after signing

## Testing

To generate test vectors for testing signature verification:

```bash
# Generate Ed25519 keypair
openssl genpkey -algorithm ed25519 -out private.pem
openssl pkey -in private.pem -pubout -out public.pem

# Extract raw public key (32 bytes)
openssl pkey -outform DER -pubout -in private.pem | tail -c 32 | xxd -p

# Extract raw private key (32 bytes)
openssl pkey -outform DER -in private.pem | tail -c 32 | xxd -p
```

Then use the examples above to generate and verify test signatures.
