#[cfg(test)]
mod test {
    extern crate std;

    use crate::{AccountFactoryContract, AccountFactoryContractClient};
    use ephemeral_account::{AccountStatus, EphemeralAccountContractClient};
    use soroban_sdk::{testutils::Address as _, Address, Bytes, BytesN, Env};
    use std::{fs, path::PathBuf, process::Command, sync::OnceLock};

    static EPHEMERAL_WASM: OnceLock<std::vec::Vec<u8>> = OnceLock::new();

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn load_ephemeral_account_wasm() -> std::vec::Vec<u8> {
        EPHEMERAL_WASM
            .get_or_init(|| {
                let workspace_root = workspace_root();
                let target_dir = workspace_root.join("target/account_factory_test_wasm");

                let status = Command::new("cargo")
                    .args([
                        "build",
                        "-p",
                        "ephemeral_account",
                        "--target",
                        "wasm32v1-none",
                        "--release",
                        "--target-dir",
                        "target/account_factory_test_wasm",
                    ])
                    .current_dir(&workspace_root)
                    .status()
                    .expect("failed to build ephemeral_account wasm");

                assert!(status.success(), "ephemeral_account wasm build failed");

                fs::read(target_dir.join("wasm32v1-none/release/ephemeral_account.wasm"))
                    .expect("failed to read ephemeral_account wasm")
            })
            .clone()
    }

    fn upload_ephemeral_wasm(env: &Env) -> BytesN<32> {
        let wasm = load_ephemeral_account_wasm();
        env.deployer()
            .upload_contract_wasm(Bytes::from_slice(env, &wasm))
    }

    fn setup_factory(env: &Env) -> AccountFactoryContractClient<'static> {
        let factory_id = env.register(AccountFactoryContract, ());
        let client = AccountFactoryContractClient::new(env, &factory_id);

        let admin = Address::generate(env);
        let wasm_hash = upload_ephemeral_wasm(env);
        client.initialize(&admin, &wasm_hash);

        client
    }

    #[test]
    fn test_create_account_deploys_and_initializes_ephemeral_account() {
        let env = Env::default();
        env.mock_all_auths();

        let client = setup_factory(&env);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let native_transfer = Address::generate(&env);
        let claim_verifier = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        let account_address = client.create_account(
            &creator,
            &expiry_ledger,
            &recovery,
            &native_transfer,
            &claim_verifier,
        );

        let account_client = EphemeralAccountContractClient::new(&env, &account_address);
        let info = account_client.get_info();

        assert_eq!(account_client.get_status(), AccountStatus::Active);
        assert_eq!(info.creator, creator);
        assert_eq!(info.expiry_ledger, expiry_ledger);
        assert_eq!(info.recovery_address, recovery);
        assert_eq!(info.payment_count, 0);
        assert!(!info.payment_received);
        assert_eq!(info.swept_to, None);
    }

    #[test]
    fn test_create_account_returns_valid_deployed_contract_address() {
        let env = Env::default();
        env.mock_all_auths();

        let client = setup_factory(&env);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let native_transfer = Address::generate(&env);
        let claim_verifier = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        let account_address = client.create_account(
            &creator,
            &expiry_ledger,
            &recovery,
            &native_transfer,
            &claim_verifier,
        );

        let account_client = EphemeralAccountContractClient::new(&env, &account_address);
        assert_eq!(account_client.get_status(), AccountStatus::Active);
        assert!(!account_client.is_expired());
    }

    #[test]
    fn test_deployed_account_is_tracked_by_creator_and_globally() {
        let env = Env::default();
        env.mock_all_auths();

        let client = setup_factory(&env);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let native_transfer = Address::generate(&env);
        let claim_verifier = Address::generate(&env);
        let expiry_ledger = env.ledger().sequence() + 1000;

        let account_address = client.create_account(
            &creator,
            &expiry_ledger,
            &recovery,
            &native_transfer,
            &claim_verifier,
        );

        let by_creator = client.get_accounts_by_creator(&creator);
        let all_accounts = client.get_all_accounts();

        assert_eq!(by_creator.len(), 1);
        assert_eq!(by_creator.get(0).unwrap(), account_address);
        assert_eq!(all_accounts.len(), 1);
        assert_eq!(all_accounts.get(0).unwrap(), account_address);
    }

    #[test]
    fn test_two_accounts_for_same_creator_are_both_tracked() {
        let env = Env::default();
        env.mock_all_auths();

        let client = setup_factory(&env);
        let creator = Address::generate(&env);
        let recovery = Address::generate(&env);
        let native_transfer = Address::generate(&env);
        let claim_verifier = Address::generate(&env);

        let account_a = client.create_account(
            &creator,
            &(env.ledger().sequence() + 1000),
            &recovery,
            &native_transfer,
            &claim_verifier,
        );
        let account_b = client.create_account(
            &creator,
            &(env.ledger().sequence() + 2000),
            &recovery,
            &native_transfer,
            &claim_verifier,
        );

        let by_creator = client.get_accounts_by_creator(&creator);
        let all_accounts = client.get_all_accounts();

        assert_eq!(by_creator.len(), 2);
        assert_eq!(by_creator.get(0).unwrap(), account_a);
        assert_eq!(by_creator.get(1).unwrap(), account_b);

        assert_eq!(all_accounts.len(), 2);
        assert_eq!(all_accounts.get(0).unwrap(), account_a);
        assert_eq!(all_accounts.get(1).unwrap(), account_b);
    }
}
