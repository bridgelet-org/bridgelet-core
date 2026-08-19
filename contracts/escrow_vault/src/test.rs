#[cfg(test)]
mod test {
    use crate::{EscrowVault, EscrowVaultClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token, Address, Env,
    };

    fn create_token<'a>(
        env: &'a Env,
        admin: &'a Address,
    ) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
        let contract_address = env.register_stellar_asset_contract_v2(admin.clone());
        let client = token::Client::new(env, &contract_address.address());
        let admin_client = token::StellarAssetClient::new(env, &contract_address.address());
        (contract_address.address(), client, admin_client)
    }

    fn create_env() -> Env {
        let env = Env::default();
        env.ledger().with_mut(|li| {
            li.sequence_number = 1_000;
            li.min_persistent_entry_ttl = 50;
            li.min_temp_entry_ttl = 50;
            li.max_entry_ttl = 600_000;
        });
        env
    }

    fn setup() -> (
        Env,
        EscrowVaultClient<'static>,
        Address, // depositor
        Address, // beneficiary
        Address, // asset
        Address, // contract_id
    ) {
        let env = create_env();
        env.mock_all_auths();

        let contract_id = env.register(EscrowVault, ());
        let client = EscrowVaultClient::new(&env, &contract_id);

        let token_admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let beneficiary = Address::generate(&env);

        let (asset, _, token_admin_client) = create_token(&env, &token_admin);

        token_admin_client.mint(&depositor, &10_000i128);

        (env, client, depositor, beneficiary, asset, contract_id)
    }

    #[test]
    fn test_successful_release() {
        let (env, client, depositor, beneficiary, asset, contract_id) = setup();

        let amount = 1_000;
        let release_ledger = 1_200;

        let id = client.open(&depositor, &beneficiary, &asset, &amount, &release_ledger);
        assert_eq!(id, 1);

        let token = token::Client::new(&env, &asset);
        assert_eq!(token.balance(&depositor), 9_000);
        assert_eq!(token.balance(&contract_id), 1_000);

        let entry = client.get(&id).unwrap();
        assert_eq!(entry.depositor, depositor);
        assert_eq!(entry.beneficiary, beneficiary);
        assert_eq!(entry.amount, amount);
        assert_eq!(entry.release_ledger, release_ledger);

        // Advance ledger past release_ledger
        env.ledger().with_mut(|l| {
            l.sequence_number = 1_200;
        });

        client.release(&id);

        assert_eq!(token.balance(&contract_id), 0);
        assert_eq!(token.balance(&beneficiary), 1_000);

        assert!(client.get(&id).is_none());
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")] // TooEarlyToRelease
    fn test_early_release_rejection() {
        let (_env, client, depositor, beneficiary, asset, _) = setup();

        let amount = 1_000;
        let release_ledger = 1_200;

        let id = client.open(&depositor, &beneficiary, &asset, &amount, &release_ledger);

        // Current ledger is 1000 < 1200
        client.release(&id);
    }

    #[test]
    fn test_cancel_before_release() {
        let (env, client, depositor, beneficiary, asset, contract_id) = setup();

        let amount = 1_000;
        let release_ledger = 1_200;

        let id = client.open(&depositor, &beneficiary, &asset, &amount, &release_ledger);

        let token = token::Client::new(&env, &asset);
        assert_eq!(token.balance(&contract_id), 1_000);

        client.cancel(&depositor, &id);

        assert_eq!(token.balance(&contract_id), 0);
        assert_eq!(token.balance(&depositor), 10_000); // Fully refunded

        assert!(client.get(&id).is_none());
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #3)")] // Unauthorized
    fn test_cancel_unauthorized_user() {
        let (_env, client, depositor, beneficiary, asset, _) = setup();

        let amount = 1_000;
        let release_ledger = 1_200;

        let id = client.open(&depositor, &beneficiary, &asset, &amount, &release_ledger);

        let other = Address::generate(&env);
        client.cancel(&other, &id);
    }
}
