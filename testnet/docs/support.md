# Testnet Support

Use this page if you are integrating against the **Bridgelet testnet
deployment** and something doesn't behave as expected. For example, an
ephemeral account won't sweep, a contract ID no longer resolves, or a call
that used to work now fails.

## Is it a testnet problem?

Before you file, rule out the common causes:

1. **Check your network settings.** Compare your passphrase, RPC URL and
   contract IDs against [network-config.md](network-config.md). A wrong
   passphrase is the most common cause of `Unauthorized` errors.
2. **Check [changelog.md](changelog.md).** The contracts may have been
   redeployed with new IDs or new behavior since you last integrated.
3. **Check whether testnet was reset.** The SDF resets testnet
   periodically, and a reset wipes every account and contract. If none of
   the IDs in `network-config.md` resolve, that is almost certainly why.
4. **Look up the error code.** The troubleshooting table in
   [getting-started.md](getting-started.md#troubleshooting) covers the
   common failures of `initialize` and `record_payment`.

## Where to report

Open an issue on **https://github.com/bridgelet-org/bridgelet-core/issues**:

- Start the title with **`[testnet]`**, for example
  `[testnet] execute_sweep returns #2003 for funded ephemeral account`.
- Add the **`testnet`** label. If you can't set labels, the `[testnet]`
  prefix is enough and a maintainer will apply the label.

Keep this separate from general contract-development issues:

| Your problem | Where it goes |
|---|---|
| The deployed testnet contracts, IDs, config or behavior don't match the docs | `[testnet]` issue + `testnet` label |
| A bug in contract source, tests, build scripts or a feature request | Regular issue, no `testnet` label |
| A possible security vulnerability, on any network | **Do not open a public issue.** Contact the maintainers privately (see below). |

## What to include

Copy this into the issue body:

```markdown
**What I tried:** (function name, e.g. SweepController::execute_sweep)
**What I expected:**
**What happened:** (full error, e.g. `Error(Contract, #2003)`)

- Contract ID(s) called:
- Transaction hash (if submitted):
- Ephemeral account contract ID (if relevant):
- Approximate ledger / UTC time:
- Client: stellar-cli vX.Y / @stellar/stellar-sdk vX.Y / other
- Network passphrase used:
```

A transaction hash is the most useful single detail. You can look it up on
https://stellar.expert/explorer/testnet, and it lets a maintainer reproduce
the exact call.

**Never post secret keys (`S...`) or signing-key material**, even for
throwaway testnet accounts.

## Security issues

If you think you've found a vulnerability, such as a sweep going to an
unauthorized destination or a bypass of signature verification, report it
privately. Use GitHub's **Report a vulnerability** button on the repository's
Security tab. Do this even if you only reproduced it on testnet, because the
same code is intended for mainnet.

## Response expectations

Testnet support is best-effort and handled by the maintainers. No response
time is guaranteed. Issues that block several integrators, such as a testnet
reset that leaves the deployment empty, take priority.
