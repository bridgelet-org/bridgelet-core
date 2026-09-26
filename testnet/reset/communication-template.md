# Testnet Reset Communication Template & Post-Reset Checklist

## Announcement Template for Integrators

> **Notice: Upcoming Testnet Contract Redeployment**
> 
> Please be advised that the Bridgelet Core contracts on Stellar Testnet will be redeployed on **[DATE]** at **[TIME UTC]**.
> Updated contract IDs and WASM hashes will be published in `config/testnet_contracts.json`.

## Post-Reset Verification Checklist

- [x] Verify all deployed contract WASM hashes match release artifacts.
- [x] Update contract IDs in SDK documentation and test suites.
- [x] Confirm Friendbot funding operates cleanly for fresh test accounts.
