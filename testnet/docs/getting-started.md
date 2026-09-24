# Getting Started on Testnet

Make your first `EphemeralAccount::initialize()` and `record_payment()` calls
against the Bridgelet testnet deployment. This takes about five minutes.

Network endpoints, the passphrase and the contract IDs are listed in
[network-config.md](network-config.md).

## Read this first: why you deploy your own instance

Each EphemeralAccount contract instance represents **one** ephemeral
account, and `initialize()` can be called only once per instance. The shared
instance `CB7Z22TXR6ZKG7MDXIGJL6QVQQ3L3DQOFR6JV325NZK4CTYWFPVZBBP3` is
already initialized, so if you call `initialize()` on it you get
`Error(Contract, #1000)` (`AlreadyInitialized`). And only that instance's
`authorized_controller` can call `record_payment()` on it.

Instead, create a fresh instance from the EphemeralAccount WASM that is
already installed on testnet. You don't need to build anything:

```
EPHEMERAL_WASM_HASH=5e667ea0687341bdccc81538492143b573777dd04b2450cdeb89b02cee62c58e
```

This is the WASM behind the deployed contract ID. Its interface is:

```rust
initialize(creator: Address, expiry_ledger: u32, recovery_address: Address,
           authorized_controller: Address, admin: Address) -> Result<(), Error>
record_payment(amount: i128, asset: Address) -> Result<(), Error>
```

> The `initialize` on `main` also takes an `authorized_signer: BytesN<32>`
> argument, but that build has not been deployed to testnet yet. Use the
> five-argument signature above against testnet.

In this walkthrough your own key plays every role: creator, recovery
address, authorized controller and admin. That way you can sign
`record_payment` yourself. In production, `authorized_controller` is the
SweepController contract, which records payments and runs sweeps.

---

## Option A: stellar-cli

Prerequisites: [stellar-cli](https://developers.stellar.org/docs/tools/cli)
v22 or later (`cargo install --locked stellar-cli`), plus `curl` and `jq`.

### 1. Create and fund a testnet identity

```bash
stellar keys generate alice --network testnet --fund
ME=$(stellar keys address alice)
```

### 2. Deploy your own EphemeralAccount instance

```bash
EA_ID=$(stellar contract deploy \
  --wasm-hash 5e667ea0687341bdccc81538492143b573777dd04b2450cdeb89b02cee62c58e \
  --source alice \
  --network testnet)
echo "EphemeralAccount: $EA_ID"
```

### 3. Call `initialize()`

`expiry_ledger` must be later than the current ledger. This sets it about
14 hours ahead:

```bash
LATEST=$(curl -s https://soroban-testnet.stellar.org \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger"}' | jq .result.sequence)

stellar contract invoke --id "$EA_ID" --source alice --network testnet \
  -- initialize \
  --creator "$ME" \
  --expiry_ledger $((LATEST + 10000)) \
  --recovery_address "$ME" \
  --authorized_controller "$ME" \
  --admin "$ME"
```

### 4. Call `record_payment()`

Record a 100 XLM payment. Amounts are in stroops (1 XLM = 10,000,000
stroops), and `asset` is the asset's Stellar Asset Contract address:

```bash
XLM=$(stellar contract id asset --asset native --network testnet)
# -> CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC

stellar contract invoke --id "$EA_ID" --source alice --network testnet \
  -- record_payment \
  --amount 1000000000 \
  --asset "$XLM"
```

### 5. Check the result

```bash
stellar contract invoke --id "$EA_ID" --source alice --network testnet -- get_info
```

You should see `"status": 1` (`PaymentReceived`), `"payment_count": 1`,
and your payment in `payments`.

---

## Option B: JavaScript SDK

Prerequisites: Node 18 or later and `npm i @stellar/stellar-sdk`. Save the
script as `quickstart.mjs` and run it with `node quickstart.mjs`.

```js
import {
  Keypair, Networks, TransactionBuilder, Operation, Address, Asset,
  Contract, nativeToScVal, scValToNative, rpc,
} from '@stellar/stellar-sdk';
import { randomBytes } from 'node:crypto';

const PASSPHRASE = Networks.TESTNET; // "Test SDF Network ; September 2015"
const server = new rpc.Server('https://soroban-testnet.stellar.org');
const EPHEMERAL_WASM_HASH =
  '5e667ea0687341bdccc81538492143b573777dd04b2450cdeb89b02cee62c58e';

// 1. Create and fund a throwaway key (swap in Keypair.fromSecret(...) to reuse one)
const me = Keypair.random();
await fetch(`https://friendbot.stellar.org/?addr=${me.publicKey()}`);

// Build, simulate, sign, submit and wait for a single operation
async function submit(op) {
  const source = await server.getAccount(me.publicKey());
  let tx = new TransactionBuilder(source, { fee: '1000000', networkPassphrase: PASSPHRASE })
    .addOperation(op).setTimeout(60).build();
  tx = await server.prepareTransaction(tx); // simulation + resource fees + auth
  tx.sign(me);
  const sent = await server.sendTransaction(tx);
  if (sent.status === 'ERROR') throw new Error(JSON.stringify(sent.errorResult));
  const res = await server.pollTransaction(sent.hash);
  if (res.status !== 'SUCCESS') throw new Error(`tx ${sent.hash}: ${res.status}`);
  return res.returnValue && scValToNative(res.returnValue);
}

// 2. Deploy a fresh EphemeralAccount instance from the installed WASM
const contractId = (await submit(Operation.createCustomContract({
  address: new Address(me.publicKey()),
  wasmHash: Buffer.from(EPHEMERAL_WASM_HASH, 'hex'),
  salt: randomBytes(32),
}))).toString();
const ea = new Contract(contractId);
console.log('EphemeralAccount:', contractId);

// 3. initialize(creator, expiry_ledger, recovery_address, authorized_controller, admin)
const { sequence } = await server.getLatestLedger();
const meAddr = nativeToScVal(me.publicKey(), { type: 'address' });
await submit(ea.call('initialize',
  meAddr,                                           // creator
  nativeToScVal(sequence + 10_000, { type: 'u32' }), // expiry_ledger (~14h)
  meAddr,                                           // recovery_address
  meAddr,                                           // authorized_controller
  meAddr,                                           // admin
));
console.log('initialize: ok');

// 4. record_payment(amount, asset): 100 XLM, in stroops
const xlm = Asset.native().contractId(PASSPHRASE);
await submit(ea.call('record_payment',
  nativeToScVal(1_000_000_000n, { type: 'i128' }),
  nativeToScVal(xlm, { type: 'address' }),
));
console.log('record_payment: ok');

// 5. Read back with a simulation (read-only, no fee)
const read = new TransactionBuilder(await server.getAccount(me.publicKey()),
  { fee: '100', networkPassphrase: PASSPHRASE })
  .addOperation(ea.call('get_info')).setTimeout(30).build();
const sim = await server.simulateTransaction(read);
console.log(scValToNative(sim.result.retval));
```

Expected output (IDs will differ):

```
EphemeralAccount: CBJK5JAUX5GMMHWQ4K5ZOCNZ6MKNFEGDTJRLATJGJJ7DCLY5BIT7PLQN
initialize: ok
record_payment: ok
{ creator: 'G...', expiry_ledger: 4844034, payment_count: 1, payment_received: true,
  payments: [ { amount: 1000000000n, asset: 'CDLZ...CYSC', timestamp: ... } ],
  recovery_address: 'G...', status: 1, swept_to: undefined }
```

This script was run successfully against testnet on 2026-09-23.

---

## Troubleshooting

`record_payment` only records a payment on-chain. It does not move funds and
does not check the account's token balance. Detecting the inbound payment,
for example through Horizon, is the caller's job.

| Error | Meaning | Fix |
|---|---|---|
| `Error(Contract, #1000)` | `AlreadyInitialized` | You called `initialize` on an instance that was already initialized, such as the shared deployed ID. Deploy your own instance (step 2). |
| `Error(Contract, #1001)` | `NotInitialized` | Call `initialize` before `record_payment`. |
| `Error(Contract, #1003)` | `InvalidAmount` | `amount` must be greater than 0. |
| `Error(Contract, #1004)` | `InvalidExpiry` | `expiry_ledger` must be later than the current ledger. Fetch the latest ledger again and add a margin. |
| `Error(Contract, #1007)` | `Unauthorized` | The transaction wasn't signed by the `authorized_controller`, or the contract rejected the network. Make sure you are on testnet. |
| `Error(Contract, #1012)` | `DuplicateAsset` | A payment for this asset is already recorded. Each instance accepts one payment per asset. |
| `Error(Contract, #1013)` | `TooManyPayments` | An instance holds at most 10 assets. |
| `Error(Auth, InvalidAction)` | The signer isn't the `creator` (on `initialize`) or the `authorized_controller` (on `record_payment`) | Use the same key you passed for those arguments. |
| `txBadSeq` / account not found | The source account isn't funded | Run friendbot again. |

The full list of error codes is in
[`contracts/ephemeral_account/src/errors.rs`](../../contracts/ephemeral_account/src/errors.rs).

## Next steps

- Go through SweepController so that `authorized_controller` is the
  SweepController contract (`CBEU4X5MNGOECBSTNEUFMBALH2YI5YV4UIH7YXRNOLR2DNLZQD4Z5KWE`)
  rather than your own key. See [docs/api-reference.md](../../docs/api-reference.md).
- For signing sweep authorizations, see
  [docs/SIGNATURE_FORMAT.md](../../docs/SIGNATURE_FORMAT.md).
