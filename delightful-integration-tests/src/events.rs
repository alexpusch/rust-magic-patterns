use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToyOrdered {
    pub timestamp: DateTime<Utc>,
    pub toy_id: i32,
    pub customer_id: i32,
}

impl ToyOrdered {
    pub fn new(timestamp: DateTime<Utc>, toy_id: i32, customer_id: i32) -> Self {
        Self {
            timestamp,
            toy_id,
            customer_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToyReviewed {
    pub timestamp: DateTime<Utc>,
    pub toy_id: i32,
    pub customer_id: i32,
    pub score: i32,
}

impl ToyReviewed {
    pub fn new(timestamp: DateTime<Utc>, toy_id: i32, customer_id: i32, score: i32) -> Self {
        Self {
            timestamp,
            toy_id,
            customer_id,
            score,
        }
    }
}
