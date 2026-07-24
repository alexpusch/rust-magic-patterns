use sqlx::PgPool;

use crate::events::ToyOrdered;

pub struct ToyOrdersService {
    pool: PgPool,
}

impl ToyOrdersService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn handle_toy_order(&self, order: ToyOrdered) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO toy_orders (ordered_at, toy_id, customer_id) VALUES ($1, $2, $3)")
            .bind(order.timestamp)
            .bind(order.toy_id)
            .bind(order.customer_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
