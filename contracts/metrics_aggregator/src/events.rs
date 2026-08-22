use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

// ── Event payloads ───────────────────────────────────────────────────────────

/// Emitted when [`MetricsAggregator::initialize`] is called successfully.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Initialized {
    pub admin: Address,
}

/// Emitted when a writer is authorized.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriterAuthorized {
    pub writer: Address,
    pub admin: Address,
}

/// Emitted when a writer's authorization is revoked.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriterRevoked {
    pub writer: Address,
    pub admin: Address,
}

/// Emitted when a metric is incremented.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetricIncremented {
    /// The authorized writer that performed the increment.
    pub writer: Address,
    /// The metric that was incremented (≤ 9 bytes for `symbol_short!` compat).
    pub metric: Symbol,
    /// The amount added by this call.
    pub amount: i128,
    /// The running total *after* the increment was applied.
    pub total: i128,
}

// ── Emit helpers ─────────────────────────────────────────────────────────────

pub fn emit_initialized(env: &Env, admin: Address) {
    env.events()
        .publish((symbol_short!("init"),), Initialized { admin });
}

pub fn emit_writer_authorized(env: &Env, writer: Address, admin: Address) {
    env.events().publish(
        (symbol_short!("writer"),),
        WriterAuthorized { writer, admin },
    );
}

pub fn emit_writer_revoked(env: &Env, writer: Address, admin: Address) {
    env.events()
        .publish((symbol_short!("revoked"),), WriterRevoked { writer, admin });
}

pub fn emit_metric_incremented(
    env: &Env,
    writer: Address,
    metric: Symbol,
    amount: i128,
    total: i128,
) {
    env.events().publish(
        (symbol_short!("incr"),),
        MetricIncremented {
            writer,
            metric,
            amount,
            total,
        },
    );
}
