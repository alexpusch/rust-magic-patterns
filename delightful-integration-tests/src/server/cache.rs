use crate::server::api_server::{
    TopRatedToysQuery, TopRatedToysResponse, TopToysQuery, TopToysResponse,
};
use redis::{AsyncCommands, aio::MultiplexedConnection};
use serde::{Serialize, de::DeserializeOwned};
use tracing::warn;

#[derive(Clone)]
pub struct RedisCache {
    redis_conn: MultiplexedConnection,
    ttl_secs: u64,
}

impl RedisCache {
    pub fn new(redis_conn: MultiplexedConnection, ttl_secs: u64) -> Self {
        Self {
            redis_conn,
            ttl_secs,
        }
    }

    pub async fn get_json<T>(&mut self, key: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        let payload: Option<String> = match self.redis_conn.get(key).await {
            Ok(payload) => payload,
            Err(error) => {
                warn!(%error, %key, "Failed to read cached response from Redis");
                return None;
            }
        };

        payload.and_then(|payload| match serde_json::from_str(&payload) {
            Ok(value) => Some(value),
            Err(error) => {
                warn!(%error, %key, "Failed to deserialize cached response");
                None
            }
        })
    }

    pub async fn set_json<T>(&mut self, key: &str, value: &T)
    where
        T: Serialize,
    {
        let payload = match serde_json::to_string(value) {
            Ok(payload) => payload,
            Err(error) => {
                warn!(%error, %key, "Failed to serialize response for Redis cache");
                return;
            }
        };

        if let Err(error) = self
            .redis_conn
            .set_ex::<_, _, ()>(key, payload, self.ttl_secs)
            .await
        {
            warn!(%error, %key, "Failed to write cached response to Redis");
        }
    }
}

pub async fn get_top_toys(cache: &mut RedisCache, query: &TopToysQuery) -> Option<TopToysResponse> {
    cache
        .get_json::<TopToysResponse>(&top_toys_key(query))
        .await
}

pub async fn set_top_toys(cache: &mut RedisCache, query: &TopToysQuery, value: &TopToysResponse) {
    cache.set_json(&top_toys_key(query), value).await;
}

pub async fn get_top_rated_toys(
    cache: &mut RedisCache,
    query: &TopRatedToysQuery,
) -> Option<TopRatedToysResponse> {
    cache
        .get_json::<TopRatedToysResponse>(&top_rated_toys_key(query))
        .await
}

pub async fn set_top_rated_toys(
    cache: &mut RedisCache,
    query: &TopRatedToysQuery,
    value: &TopRatedToysResponse,
) {
    cache.set_json(&top_rated_toys_key(query), value).await;
}

fn top_toys_key(query: &TopToysQuery) -> String {
    format!(
        "http_cache:orders:top:{}:{}:{}",
        query.start.to_rfc3339(),
        query.end.to_rfc3339(),
        query.count
    )
}

fn top_rated_toys_key(query: &TopRatedToysQuery) -> String {
    format!(
        "http_cache:reviews:top_rated:{}:{}:{}",
        query.start.to_rfc3339(),
        query.end.to_rfc3339(),
        query.count
    )
}
