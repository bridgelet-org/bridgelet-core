use super::*;
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env, Symbol};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Deploy and initialize the contract, returning the client and its admin.
fn setup() -> (Env, MetricsAggregatorClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(MetricsAggregator, ());
    let client = MetricsAggregatorClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

/// Deploy, initialize, and additionally authorize one writer.
fn setup_with_writer() -> (Env, MetricsAggregatorClient<'static>, Address, Address) {
    let (env, client, admin) = setup();
    let writer = Address::generate(&env);
    client.authorize_writer(&admin, &writer);
    (env, client, admin, writer)
}

/// Deploy *without* initializing.
fn setup_uninitialized() -> (Env, MetricsAggregatorClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(MetricsAggregator, ());
    let client = MetricsAggregatorClient::new(&env, &contract_id);
    (env, client)
}

// ── initialize ───────────────────────────────────────────────────────────────

#[test]
fn initialize_succeeds_once() {
    let (env, client) = setup_uninitialized();
    let admin = Address::generate(&env);

    let result = client.try_initialize(&admin);
    assert!(result.is_ok());
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
fn initialize_twice_returns_error() {
    let (_, client, admin) = setup();

    let result = client.try_initialize(&admin);
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::AlreadyInitialized,
        "second initialize must return AlreadyInitialized"
    );
}

#[test]
fn get_admin_before_initialize_returns_none() {
    let (_, client) = setup_uninitialized();
    assert_eq!(client.get_admin(), None);
}

// ── authorize_writer ─────────────────────────────────────────────────────────

#[test]
fn authorize_writer_succeeds() {
    let (env, client, admin) = setup();
    let writer = Address::generate(&env);

    let result = client.try_authorize_writer(&admin, &writer);
    assert!(result.is_ok());
    assert!(client.is_writer(&writer));
}

#[test]
fn authorize_writer_non_admin_returns_unauthorized() {
    let (env, client, _admin) = setup();
    let intruder = Address::generate(&env);
    let writer = Address::generate(&env);

    let result = client.try_authorize_writer(&intruder, &writer);
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
    assert!(
        !client.is_writer(&writer),
        "a non-admin call must not authorize anyone"
    );
}

#[test]
fn authorize_writer_before_initialize_returns_not_initialized() {
    let (env, client) = setup_uninitialized();
    let admin = Address::generate(&env);
    let writer = Address::generate(&env);

    let result = client.try_authorize_writer(&admin, &writer);
    assert_eq!(result.unwrap_err().unwrap(), Error::NotInitialized);
}

/// Authorizing the same writer twice must succeed (idempotent).
#[test]
fn authorize_writer_idempotent() {
    let (env, client, admin) = setup();
    let writer = Address::generate(&env);

    client.authorize_writer(&admin, &writer);
    let result = client.try_authorize_writer(&admin, &writer);
    assert!(result.is_ok());
    assert!(client.is_writer(&writer));
}

#[test]
fn is_writer_false_for_unknown_address() {
    let (env, client, _admin) = setup();
    let stranger = Address::generate(&env);
    assert!(!client.is_writer(&stranger));
}

// ── revoke_writer ────────────────────────────────────────────────────────────

#[test]
fn revoke_writer_blocks_further_increments() {
    let (_, client, admin, writer) = setup_with_writer();
    let metric = symbol_short!("sweeps");

    client.increment(&writer, &metric, &1i128);
    client.revoke_writer(&admin, &writer);

    assert!(!client.is_writer(&writer));
    let result = client.try_increment(&writer, &metric, &1i128);
    assert_eq!(result.unwrap_err().unwrap(), Error::UnauthorizedWriter);
    assert_eq!(
        client.get(&metric),
        1,
        "revocation must not roll back totals already recorded"
    );
}

#[test]
fn revoke_writer_non_admin_returns_unauthorized() {
    let (env, client, _admin, writer) = setup_with_writer();
    let intruder = Address::generate(&env);

    let result = client.try_revoke_writer(&intruder, &writer);
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
    assert!(client.is_writer(&writer), "writer must still be authorized");
}

/// Revoking an address that was never authorized is a no-op, not an error.
#[test]
fn revoke_writer_unknown_address_is_noop() {
    let (env, client, admin) = setup();
    let stranger = Address::generate(&env);

    let result = client.try_revoke_writer(&admin, &stranger);
    assert!(result.is_ok());
}

#[test]
fn revoke_then_reauthorize_restores_access() {
    let (_, client, admin, writer) = setup_with_writer();
    let metric = symbol_short!("created");

    client.revoke_writer(&admin, &writer);
    client.authorize_writer(&admin, &writer);

    client.increment(&writer, &metric, &7i128);
    assert_eq!(client.get(&metric), 7);
}

// ── increment: authorization ─────────────────────────────────────────────────

#[test]
fn increment_by_authorized_writer_succeeds() {
    let (_, client, _admin, writer) = setup_with_writer();
    let metric = symbol_short!("created");

    let result = client.try_increment(&writer, &metric, &1i128);
    assert!(result.is_ok());
    assert_eq!(client.get(&metric), 1);
}

#[test]
fn increment_by_unauthorized_writer_returns_error() {
    let (env, client, _admin) = setup();
    let intruder = Address::generate(&env);
    let metric = symbol_short!("created");

    let result = client.try_increment(&intruder, &metric, &100i128);
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::UnauthorizedWriter,
        "an unauthorized address must not be able to increment"
    );
    assert_eq!(
        client.get(&metric),
        0,
        "a rejected increment must leave the counter untouched"
    );
}

/// The admin is not implicitly a writer — it must authorize itself first.
#[test]
fn increment_by_admin_without_authorization_returns_error() {
    let (_, client, admin) = setup();
    let metric = symbol_short!("created");

    let result = client.try_increment(&admin, &metric, &1i128);
    assert_eq!(result.unwrap_err().unwrap(), Error::UnauthorizedWriter);
}

/// An authorized writer is not enough on its own: the call must also carry
/// that writer's authorization, so a third party cannot increment on its
/// behalf.
#[test]
#[should_panic(expected = "InvalidAction")]
fn increment_without_writer_auth_panics() {
    let (env, client, _admin, writer) = setup_with_writer();

    // Stop mocking auths: the call now carries no signatures at all.
    env.set_auths(&[]);
    client.increment(&writer, &symbol_short!("created"), &1i128);
}

#[test]
fn increment_before_initialize_returns_not_initialized() {
    let (env, client) = setup_uninitialized();
    let writer = Address::generate(&env);

    let result = client.try_increment(&writer, &symbol_short!("created"), &1i128);
    assert_eq!(result.unwrap_err().unwrap(), Error::NotInitialized);
}

// ── increment: amount validation ─────────────────────────────────────────────

#[test]
fn increment_zero_amount_returns_invalid_amount() {
    let (_, client, _admin, writer) = setup_with_writer();
    let metric = symbol_short!("created");

    let result = client.try_increment(&writer, &metric, &0i128);
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidAmount);
    assert_eq!(client.get(&metric), 0);
}

#[test]
fn increment_negative_amount_returns_invalid_amount() {
    let (_, client, _admin, writer) = setup_with_writer();
    let metric = symbol_short!("volume");

    client.increment(&writer, &metric, &50i128);

    let result = client.try_increment(&writer, &metric, &-10i128);
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::InvalidAmount,
        "counters are monotonic; a negative amount must be rejected"
    );
    assert_eq!(client.get(&metric), 50, "the total must not move");
}

#[test]
fn increment_overflow_returns_error_and_preserves_total() {
    let (_, client, _admin, writer) = setup_with_writer();
    let metric = symbol_short!("volume");

    client.increment(&writer, &metric, &i128::MAX);
    assert_eq!(client.get(&metric), i128::MAX);

    let result = client.try_increment(&writer, &metric, &1i128);
    assert_eq!(result.unwrap_err().unwrap(), Error::CounterOverflow);
    assert_eq!(
        client.get(&metric),
        i128::MAX,
        "an overflowing increment must leave the stored total unchanged"
    );
}

// ── get ──────────────────────────────────────────────────────────────────────

/// Acceptance criterion: an untouched metric reads as 0, it does not error.
#[test]
fn get_unknown_metric_returns_zero() {
    let (_, client, _admin) = setup();
    assert_eq!(client.get(&symbol_short!("nothing")), 0);
}

#[test]
fn get_unknown_metric_before_initialize_returns_zero() {
    let (_, client) = setup_uninitialized();
    assert_eq!(
        client.get(&symbol_short!("nothing")),
        0,
        "reads are unrestricted and must not depend on initialization"
    );
}

#[test]
fn get_does_not_mutate_the_counter() {
    let (_, client, _admin, writer) = setup_with_writer();
    let metric = symbol_short!("created");

    client.increment(&writer, &metric, &3i128);
    assert_eq!(client.get(&metric), 3);
    assert_eq!(client.get(&metric), 3);
    assert_eq!(client.get(&metric), 3);
}

// ── Accumulation ─────────────────────────────────────────────────────────────

#[test]
fn repeated_increments_accumulate() {
    let (_, client, _admin, writer) = setup_with_writer();
    let metric = symbol_short!("sweeps");

    client.increment(&writer, &metric, &5i128);
    client.increment(&writer, &metric, &5i128);
    client.increment(&writer, &metric, &5i128);

    assert_eq!(client.get(&metric), 15);
}

/// Concurrent-style load: many small increments from a single writer, as a
/// stream of independent transactions would produce.
#[test]
fn many_sequential_increments_accumulate_exactly() {
    let (_, client, _admin, writer) = setup_with_writer();
    let metric = symbol_short!("created");

    for _ in 0..250 {
        client.increment(&writer, &metric, &1i128);
    }

    assert_eq!(client.get(&metric), 250);
}

/// Concurrent-style load: several writers hammering the *same* metric, their
/// calls interleaved. Every increment must land exactly once.
#[test]
fn interleaved_increments_from_many_writers_accumulate() {
    let (env, client, admin) = setup();
    let metric = symbol_short!("sweeps");

    let writers = [
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];
    for writer in writers.iter() {
        client.authorize_writer(&admin, writer);
    }

    // 50 rounds x 4 writers x 3 each = 600.
    for _ in 0..50 {
        for writer in writers.iter() {
            client.increment(writer, &metric, &3i128);
        }
    }

    assert_eq!(client.get(&metric), 600);
}

/// Interleaved writes across *different* metrics must not bleed into each
/// other.
#[test]
fn interleaved_increments_across_metrics_stay_independent() {
    let (_, client, _admin, writer) = setup_with_writer();
    let created = symbol_short!("created");
    let swept = symbol_short!("swept");
    let expired = symbol_short!("expired");

    for _ in 0..20 {
        client.increment(&writer, &created, &1i128);
        client.increment(&writer, &swept, &2i128);
        client.increment(&writer, &expired, &4i128);
    }

    assert_eq!(client.get(&created), 20);
    assert_eq!(client.get(&swept), 40);
    assert_eq!(client.get(&expired), 80);
}

/// A rejected increment interleaved with valid ones must not corrupt the
/// running total.
#[test]
fn rejected_increments_interleaved_with_valid_ones_do_not_corrupt_total() {
    let (env, client, _admin, writer) = setup_with_writer();
    let intruder = Address::generate(&env);
    let metric = symbol_short!("volume");

    for _ in 0..10 {
        client.increment(&writer, &metric, &10i128);
        assert!(client
            .try_increment(&intruder, &metric, &1_000i128)
            .is_err());
        assert!(client.try_increment(&writer, &metric, &0i128).is_err());
    }

    assert_eq!(client.get(&metric), 100);
}

#[test]
fn large_amounts_accumulate_without_precision_loss() {
    let (_, client, _admin, writer) = setup_with_writer();
    let metric = Symbol::new(&client.env, "volume_usdc");
    let stroops = 10_000_000_000_000i128; // 1M USDC in stroops

    for _ in 0..100 {
        client.increment(&writer, &metric, &stroops);
    }

    assert_eq!(client.get(&metric), stroops * 100);
}

// ── Metric naming ────────────────────────────────────────────────────────────

/// Metrics longer than the 9-byte `symbol_short!` limit are usable too, which
/// is what makes per-asset volume metrics practical.
#[test]
fn long_metric_names_are_supported_and_distinct() {
    let (_, client, _admin, writer) = setup_with_writer();
    let usdc = Symbol::new(&client.env, "total_volume_usdc");
    let xlm = Symbol::new(&client.env, "total_volume_xlm");

    client.increment(&writer, &usdc, &111i128);
    client.increment(&writer, &xlm, &222i128);

    assert_eq!(client.get(&usdc), 111);
    assert_eq!(client.get(&xlm), 222);
}
