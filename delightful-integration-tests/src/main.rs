mod app;
mod events;
mod migration;
mod server;
mod toy_orders;
mod toy_reviews;
use std::env;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::app::App;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let postgres_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://user:password@localhost:5432/toy_analytics".to_string());
    let rabbitmq_url = env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://user:password@localhost:5672/%2f".to_string());
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379/".to_string());

    App::new(app::AppConfig {
        postgres_url,
        rabbitmq_url,
        redis_url,
        api_port: 8080,
    })
    .await?
    .run()
    .await?;

    Ok(())
}
