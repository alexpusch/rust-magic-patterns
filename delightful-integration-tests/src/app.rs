use sqlx::postgres::PgPoolOptions;
use tracing::info;

use crate::migration::run_migrations;
use crate::server::api_server::ApiServer;
use crate::toy_orders::ToyOrdersService;
use crate::{
    server::cache::RedisCache,
    toy_orders::ToyOrdersConsumer,
    toy_reviews::{ToyReviewConsumer, ToyReviewsService},
};

pub struct AppConfig {
    pub postgres_url: String,
    pub rabbitmq_url: String,
    pub redis_url: String,
    pub api_port: u16,
}

pub struct App {
    pub toy_orders_consumer: ToyOrdersConsumer,
    pub toy_reviews_consumer: ToyReviewConsumer,
    pub api_server: ApiServer,
}

impl App {
    pub async fn new(config: AppConfig) -> anyhow::Result<Self> {
        info!("Connecting to rabbitmq...");
        let rabbitmq_conn = lapin::Connection::connect(
            &config.rabbitmq_url,
            lapin::ConnectionProperties::default(),
        )
        .await?;

        info!("Connecting to database...");
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.postgres_url)
            .await?;

        info!("Running migrations...");
        run_migrations(&pool).await?;

        let redis_client = redis::Client::open(config.redis_url)?;
        let redis_conn = redis_client.get_multiplexed_async_connection().await?;

        let api_server = ApiServer::new(
            pool.clone(),
            RedisCache::new(redis_conn, 60),
            config.api_port,
        )
        .await?;

        let rabbitmq_channel = rabbitmq_conn.create_channel().await?;
        let toy_orders_service = ToyOrdersService::new(pool.clone());
        let toy_orders_consumer =
            ToyOrdersConsumer::new(rabbitmq_channel.clone(), toy_orders_service).await?;

        let toy_reviews_service = ToyReviewsService::new(pool.clone());
        let toy_reviews_consumer =
            ToyReviewConsumer::new(rabbitmq_channel, toy_reviews_service).await?;

        Ok(Self {
            toy_orders_consumer,
            toy_reviews_consumer,
            api_server,
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let orders_consumer_fut = self.toy_orders_consumer.consume();
        let reviews_consumer_fut = self.toy_reviews_consumer.consume();
        let server_fut = self.api_server.serve();

        tokio::try_join!(server_fut, orders_consumer_fut, reviews_consumer_fut)?;
        Ok(())
    }
}
