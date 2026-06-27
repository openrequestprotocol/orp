use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    migrate(&pool).await?;
    Ok(pool)
}

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS orp_users (
            email TEXT PRIMARY KEY,
            policy_json JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
        );

        CREATE TABLE IF NOT EXISTS orp_requests (
            id TEXT PRIMARY KEY,
            recipient TEXT NOT NULL,
            sender TEXT NOT NULL,
            request_json JSONB NOT NULL,
            importance TEXT NOT NULL,
            intent TEXT NOT NULL,
            state TEXT NOT NULL DEFAULT 'pending',
            transport TEXT NOT NULL DEFAULT 'native',
            confidence REAL NOT NULL DEFAULT 1.0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
        );

        CREATE INDEX IF NOT EXISTS idx_orp_requests_recipient_state
            ON orp_requests (recipient, state, created_at DESC);

        CREATE TABLE IF NOT EXISTS orp_known_senders (
            recipient TEXT NOT NULL,
            sender TEXT NOT NULL,
            first_seen TIMESTAMPTZ NOT NULL DEFAULT now(),
            PRIMARY KEY (recipient, sender)
        );

        CREATE TABLE IF NOT EXISTS orp_feedback (
            id TEXT PRIMARY KEY,
            request_id TEXT NOT NULL REFERENCES orp_requests(id) ON DELETE CASCADE,
            recipient TEXT NOT NULL,
            action TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
        );

        CREATE TABLE IF NOT EXISTS orp_reputation (
            recipient TEXT NOT NULL,
            sender TEXT NOT NULL,
            score REAL NOT NULL DEFAULT 0,
            high_claims INT NOT NULL DEFAULT 0,
            high_confirmed INT NOT NULL DEFAULT 0,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
            PRIMARY KEY (recipient, sender)
        );

        CREATE TABLE IF NOT EXISTS orp_budget_state (
            recipient TEXT NOT NULL,
            sender TEXT NOT NULL,
            high_used INT NOT NULL DEFAULT 0,
            unknown_today INT NOT NULL DEFAULT 0,
            window_start TIMESTAMPTZ NOT NULL DEFAULT now(),
            PRIMARY KEY (recipient, sender)
        );

        CREATE TABLE IF NOT EXISTS orp_delivery_queue (
            id TEXT PRIMARY KEY,
            request_json JSONB NOT NULL,
            target_endpoint TEXT NOT NULL,
            attempts INT NOT NULL DEFAULT 0,
            next_attempt TIMESTAMPTZ NOT NULL DEFAULT now(),
            last_error TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
        );

        CREATE TABLE IF NOT EXISTS orp_user_keys (
            email TEXT PRIMARY KEY,
            key_id TEXT NOT NULL,
            alg TEXT NOT NULL DEFAULT 'ed25519',
            public_key TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
        );
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
