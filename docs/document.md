 testnet/config/timeouts.md: document recommended RPC/transaction timeout and retry settings for testnet's occasional slowness
Repo Avatar
bridgelet-org/bridgelet-core
File: testnet/config/timeouts.md

Public testnet infrastructure is occasionally slower or less reliable than a production Horizon/RPC endpoint. Document recommended client timeout and retry settings for integration code/tests targeting testnet specifically, distinct from whatever a mainnet-oriented client would use.