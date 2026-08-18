use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup() -> (Env, ExpirySchedulerClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ExpiryScheduler, ());
    let client = ExpirySchedulerClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

#[test]
fn register_and_query_due_accounts() {
    let (env, client, _) = setup();
    let account = Address::generate(&env);

    client.register(&account, &100);

    assert_eq!(client.due_before(&99, &10).len(), 0);
    assert_eq!(client.due_before(&100, &10).get(0), Some(account));
}

#[test]
fn due_before_is_capped_and_returns_soonest_first() {
    let (env, client, _) = setup();
    let late = Address::generate(&env);
    let soon = Address::generate(&env);
    let middle = Address::generate(&env);

    client.register(&late, &300);
    client.register(&soon, &100);
    client.register(&middle, &200);

    let due = client.due_before(&300, &2);
    assert_eq!(due.len(), 2);
    assert_eq!(due.get(0), Some(soon));
    assert_eq!(due.get(1), Some(middle));
}

#[test]
fn empty_queries_and_zero_max_return_empty() {
    let (env, client, _) = setup();
    assert!(client.due_before(&100, &10).is_empty());

    let account = Address::generate(&env);
    client.register(&account, &100);
    assert!(client.due_before(&100, &0).is_empty());
}

#[test]
fn register_requires_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ExpiryScheduler, ());
    let client = ExpirySchedulerClient::new(&env, &contract_id);
    let account = Address::generate(&env);

    assert_eq!(
        client.try_register(&account, &100).unwrap_err().unwrap(),
        Error::NotInitialized
    );
}

#[test]
fn reregister_reschedules_and_preserves_one_entry() {
    let (env, client, _) = setup();
    let account = Address::generate(&env);

    client.register(&account, &300);
    client.register(&account, &100);

    assert_eq!(client.due_before(&100, &10).get(0), Some(account));
    assert_eq!(client.due_before(&300, &10).len(), 1);
}

#[test]
fn account_can_deregister_itself() {
    let (env, client, _) = setup();
    let account = Address::generate(&env);
    client.register(&account, &100);

    client.deregister(&account, &account);
    assert!(client.due_before(&100, &10).is_empty());
}

#[test]
fn admin_can_deregister_account() {
    let (env, client, admin) = setup();
    let account = Address::generate(&env);
    client.register(&account, &100);

    client.deregister(&admin, &account);
    assert!(client.due_before(&100, &10).is_empty());
}

#[test]
fn stranger_cannot_deregister_account() {
    let (env, client, _) = setup();
    let account = Address::generate(&env);
    let stranger = Address::generate(&env);
    client.register(&account, &100);

    assert_eq!(
        client
            .try_deregister(&stranger, &account)
            .unwrap_err()
            .unwrap(),
        Error::Unauthorized
    );
}

#[test]
fn deregistering_unknown_account_returns_error() {
    let (env, client, admin) = setup();
    let account = Address::generate(&env);

    assert_eq!(
        client
            .try_deregister(&admin, &account)
            .unwrap_err()
            .unwrap(),
        Error::NotRegistered
    );
}
