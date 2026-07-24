use crate::server::{
    analytics_service::AnalyticsService,
    cache::{self, RedisCache},
};
use axum::routing::get;
use axum::{
    Json, Router,
    extract::{Query, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use std::net::SocketAddr;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {
    pub analytics: AnalyticsService,
    pub cache: RedisCache,
}

#[derive(Deserialize)]
pub struct TopToysQuery {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub count: i64,
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow, PartialEq, Eq)]
pub struct TopToyResponse {
    pub toy_id: i32,
    pub orders: i64,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct TopToysResponse {
    pub toys: Vec<TopToyResponse>,
}

#[derive(Deserialize)]
pub struct TopRatedToysQuery {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub count: i64,
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow, PartialEq)]
pub struct TopRatedToyResponse {
    pub toy_id: i32,
    pub average_score: f64,
    pub reviews: i64,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct TopRatedToysResponse {
    pub toys: Vec<TopRatedToyResponse>,
}

pub struct ApiServer {
    pub router: Router,
    pub listener: TcpListener,
}

impl ApiServer {
    pub async fn new(pool: sqlx::PgPool, cache: RedisCache, api_port: u16) -> anyhow::Result<Self> {
        let analytics = AnalyticsService::new(pool);
        let app_state = AppState { analytics, cache };

        let router = Router::new()
            .route("/orders/top", get(get_top_toys))
            .route("/reviews/top_rated", get(get_top_rated_toys))
            .with_state(app_state);

        let addr = SocketAddr::from(([0, 0, 0, 0], api_port));
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;

        tracing::info!("Listening on {}", local_addr);

        Ok(Self { router, listener })
    }

    #[cfg(test)]
    pub fn local_address(&self) -> anyhow::Result<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    pub async fn serve(self) -> anyhow::Result<()> {
        axum::serve(self.listener, self.router).await?;

        Ok(())
    }
}

pub async fn get_top_toys(
    State(mut state): State<AppState>,
    Query(query): Query<TopToysQuery>,
) -> Result<Json<TopToysResponse>, String> {
    if let Some(response) = cache::get_top_toys(&mut state.cache, &query).await {
        return Ok(Json(response));
    }

    let response = state
        .analytics
        .top_toys(&query)
        .await
        .map_err(|e| e.to_string())?;

    cache::set_top_toys(&mut state.cache, &query, &response).await;

    Ok(Json(response))
}

pub async fn get_top_rated_toys(
    State(mut state): State<AppState>,
    Query(query): Query<TopRatedToysQuery>,
) -> Result<Json<TopRatedToysResponse>, String> {
    if let Some(response) = cache::get_top_rated_toys(&mut state.cache, &query).await {
        return Ok(Json(response));
    }

    let response = state
        .analytics
        .top_rated_toys(&query)
        .await
        .map_err(|e| e.to_string())?;

    cache::set_top_rated_toys(&mut state.cache, &query, &response).await;

    Ok(Json(response))
}
