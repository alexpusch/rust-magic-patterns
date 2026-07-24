use sqlx::PgPool;
use thiserror::Error;

use crate::events::ToyReviewed;

pub struct ToyReviewsService {
    pool: PgPool,
}

#[derive(Debug, Error)]
pub enum HandleToyReviewError {
    #[error("toy review has an invalid score")]
    InvalidScore(ToyReviewed),
    #[error("toy review does not have a matching order")]
    MissingOrder(ToyReviewed),
    #[error("failed to persist toy review")]
    Database(#[from] sqlx::Error),
}

impl ToyReviewsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn handle_toy_review(&self, review: ToyReviewed) -> Result<(), HandleToyReviewError> {
        if !(1..=5).contains(&review.score) {
            return Err(HandleToyReviewError::InvalidScore(review));
        }

        let has_ordered = customer_has_ordered_toy(
            &self.pool,
            review.customer_id,
            review.toy_id,
            review.timestamp,
        )
        .await?;

        if !has_ordered {
            return Err(HandleToyReviewError::MissingOrder(review));
        }

        sqlx::query(
            "INSERT INTO toy_reviews (reviewed_at, toy_id, customer_id, score) VALUES ($1, $2, $3, $4)",
        )
        .bind(review.timestamp)
        .bind(review.toy_id)
        .bind(review.customer_id)
        .bind(review.score)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

async fn customer_has_ordered_toy(
    pool: &PgPool,
    customer_id: i32,
    toy_id: i32,
    reviewed_at: chrono::DateTime<chrono::Utc>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        r#"
		SELECT EXISTS (
			SELECT 1
			FROM toy_orders
			WHERE toy_id = $1
			  AND customer_id = $2
			  AND ordered_at <= $3
		)
		"#,
    )
    .bind(toy_id)
    .bind(customer_id)
    .bind(reviewed_at)
    .fetch_one(pool)
    .await
}
