use crate::app::{App, AppConfig};
use lapin::Channel;
use lapin::{Connection, ConnectionProperties};
use redis::aio::MultiplexedConnection;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::SocketAddr;
use testcontainers::Image;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, core::WaitFor};
use testcontainers_modules::postgres;
use testcontainers_modules::redis::Redis;
use tokio::task::JoinHandle;

/// Define a custom testcontainer image for RabbitMQ.
/// This is only for demonstration, since testcontainers-modules already ships one.
pub struct RabbitMqImage;

impl Image for RabbitMqImage {
    fn name(&self) -> &str {
        "rabbitmq"
    }

    fn tag(&self) -> &str {
        "3.8.22-management"
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Server startup complete")]
    }
}

async fn start_rabbitmq() -> anyhow::Result<(ContainerAsync<RabbitMqImage>, String)> {
    let rabbitmq_container = RabbitMqImage.start().await?;

    // get the mapped port for RabbitMQ and construct the AMQP URL
    let amqp_port = rabbitmq_container.get_host_port_ipv4(5672).await?;
    let amqp_addr = format!("amqp://127.0.0.1:{}", amqp_port);

    Ok((rabbitmq_container, amqp_addr))
}

async fn start_postgres() -> anyhow::Result<(ContainerAsync<postgres::Postgres>, String)> {
    let pg_container = postgres::Postgres::default().start().await?;

    // get the mapped port for PostgreSQL and construct the database URL
    let pg_port = pg_container.get_host_port_ipv4(5432).await?;
    let db_url = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        pg_port
    );

    Ok((pg_container, db_url))
}

async fn start_redis() -> anyhow::Result<(ContainerAsync<Redis>, String)> {
    let redis_node = Redis::default().start().await?;

    // get the mapped port for Redis an construct Redis url
    let redis_port = redis_node.get_host_port_ipv4(6379).await?;
    let redis_url = format!("redis://127.0.0.1:{}/", redis_port);

    Ok((redis_node, redis_url))
}

/// Manage all needed resources for our integration tests. This includes containers, connections, and app instances.
pub struct TestEnv {
    _rabbitmq_container: ContainerAsync<RabbitMqImage>,
    _pg_container: ContainerAsync<postgres::Postgres>,
    _redis_container: ContainerAsync<Redis>,
    pub rabbitmq_channel: Channel,
    pub postgres_pool: PgPool,
    pub redis_conn: MultiplexedConnection,
    pub app_handle: JoinHandle<()>,
    pub api_address: SocketAddr,
}

impl TestEnv {
    pub async fn start() -> anyhow::Result<Self> {
        let (
            (_rabbitmq_container, rabbitmq_url),
            (_pg_container, postgres_url),
            (_redis_container, redis_url),
        ) = tokio::try_join!(start_rabbitmq(), start_postgres(), start_redis())?;

        let app = App::new(AppConfig {
            postgres_url: postgres_url.clone(),
            rabbitmq_url: rabbitmq_url.clone(),
            redis_url: redis_url.clone(),
            // choose random port to allow concurrent tests
            api_port: 0,
        })
        .await?;

        let api_address = app.api_server.local_address()?;

        // spawn an instance of our App in a new task. keep track of the join handle
        // so we could abort it when the test is done
        let app_handle = tokio::spawn(async move {
            if let Err(error) = app.run().await {
                eprintln!("Error running app: {:?}", error);
            }
        });

        let rabbitmq_channel = get_rabbitmq_channel(&rabbitmq_url).await?;
        let postgres_pool = get_pg_pool(&postgres_url).await?;
        let redis_conn = get_redis_conn(&redis_url).await?;

        Ok(Self {
            _rabbitmq_container: _rabbitmq_container,
            _pg_container: _pg_container,
            _redis_container: _redis_container,
            rabbitmq_channel,
            postgres_pool,
            redis_conn,
            api_address,
            app_handle,
        })
    }

    pub async fn stop_redis(&mut self) -> anyhow::Result<()> {
        self._redis_container.stop().await?;
        Ok(())
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        // Make sure the app is dropped so we won't have any handing tasks
        self.app_handle.abort();
    }
}

async fn get_rabbitmq_channel(rabbitmq_url: &str) -> anyhow::Result<Channel> {
    let rabbitmq_conn = Connection::connect(rabbitmq_url, ConnectionProperties::default()).await?;
    let rabbitmq_channel = rabbitmq_conn.create_channel().await?;
    Ok(rabbitmq_channel)
}

async fn get_pg_pool(db_url: &str) -> anyhow::Result<PgPool> {
    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await?;
    Ok(pg_pool)
}

async fn get_redis_conn(redis_url: &str) -> anyhow::Result<MultiplexedConnection> {
    let redis_client = redis::Client::open(redis_url)?;
    let redis_conn = redis_client.get_multiplexed_async_connection().await?;
    Ok(redis_conn)
}
