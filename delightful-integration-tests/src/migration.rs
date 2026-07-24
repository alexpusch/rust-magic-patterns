use sqlx::postgres::PgPool;

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS toy_orders (
            id SERIAL PRIMARY KEY,
            ordered_at TIMESTAMPTZ NOT NULL,
            toy_id INTEGER NOT NULL,
            customer_id INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        ALTER TABLE toy_orders
        ADD COLUMN IF NOT EXISTS customer_id INTEGER NOT NULL DEFAULT 0
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        ALTER TABLE toy_orders
        ALTER COLUMN customer_id DROP DEFAULT
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_toy_orders_ordered_at
            ON toy_orders (ordered_at)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_toy_orders_toy_id_ordered_at
            ON toy_orders (toy_id, ordered_at)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_toy_orders_customer_toy_ordered_at
            ON toy_orders (customer_id, toy_id, ordered_at)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS toy_reviews (
            id SERIAL PRIMARY KEY,
            reviewed_at TIMESTAMPTZ NOT NULL,
            toy_id INTEGER NOT NULL,
            customer_id INTEGER NOT NULL,
            score INTEGER NOT NULL CHECK (score BETWEEN 1 AND 5)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        ALTER TABLE toy_reviews
        ADD COLUMN IF NOT EXISTS customer_id INTEGER NOT NULL DEFAULT 0
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        ALTER TABLE toy_reviews
        ALTER COLUMN customer_id DROP DEFAULT
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_toy_reviews_reviewed_at
            ON toy_reviews (reviewed_at)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_toy_reviews_toy_id_reviewed_at
            ON toy_reviews (toy_id, reviewed_at)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_toy_reviews_customer_toy_reviewed_at
            ON toy_reviews (customer_id, toy_id, reviewed_at)
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}
