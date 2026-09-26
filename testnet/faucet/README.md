# Testnet Faucet & Account Funding Guide

Comprehensive documentation for funding testnet accounts using Friendbot and handling rate limits during integration testing.

## Friendbot Account Funding

To fund a generated testnet keypair with 10,000 test XLM:

```bash
curl "https://friendbot.stellar.org/?addr=<YOUR_PUBLIC_KEY>"
```

## Faucet Troubleshooting

- **429 Too Many Requests**: Friendbot limits requests per IP. Wait 60 seconds or use a proxy/VPN.
- **504 Gateway Timeout**: Testnet RPC node temporary overload. Retry request with exponential backoff.
