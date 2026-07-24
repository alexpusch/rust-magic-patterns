use anyhow::{Error, anyhow};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::server::api_server::{
    TopRatedToyResponse, TopRatedToysQuery, TopRatedToysResponse, TopToyResponse, TopToysQuery,
    TopToysResponse,
};

#[derive(Clone)]
pub struct AnalyticsService {
    pool: PgPool,
}

impl AnalyticsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn top_toys(&self, query: &TopToysQuery) -> Result<TopToysResponse, Error> {
        validate_range(query.start, query.end)?;

        let limit = query.count;
        if limit <= 0 {
            return Err(anyhow!("count must be greater than 0"));
        }

        let toys = sqlx::query_as::<_, TopToyResponse>(
            r#"
            SELECT toy_id, COUNT(*) AS orders
            FROM toy_orders
            WHERE ordered_at >= $1
                AND ordered_at < $2
            GROUP BY toy_id
            ORDER BY orders DESC, toy_id ASC
            LIMIT $3
            "#,
        )
        .bind(query.start)
        .bind(query.end)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(TopToysResponse { toys })
    }

    pub async fn top_rated_toys(
        &self,
        query: &TopRatedToysQuery,
    ) -> Result<TopRatedToysResponse, Error> {
        validate_range(query.start, query.end)?;

        let limit = query.count;
        if limit <= 0 {
            return Err(anyhow!("count must be greater than 0"));
        }

        let toys = sqlx::query_as::<_, TopRatedToyResponse>(
            r#"
            SELECT
                toy_id,
                AVG(score)::DOUBLE PRECISION AS average_score,
                COUNT(*) AS reviews
            FROM toy_reviews
            WHERE reviewed_at >= $1
                AND reviewed_at < $2
            GROUP BY toy_id
            ORDER BY average_score DESC, reviews DESC, toy_id ASC
            LIMIT $3
            "#,
        )
        .bind(query.start)
        .bind(query.end)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(TopRatedToysResponse { toys })
    }
}

fn validate_range(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<(), Error> {
    if start >= end {
        return Err(anyhow!("start must be before end"));
    }

    Ok(())
}
