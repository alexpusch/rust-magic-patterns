use crate::events::{ToyOrdered, ToyReviewed};
use crate::server::api_server::{TopRatedToysResponse, TopToyResponse, TopToysResponse};
use crate::tests::env::TestEnv;
use crate::toy_orders::TOY_ORDERS_QUEUE;
use crate::toy_reviews::TOY_REVIEWS_QUEUE;
use std::net::SocketAddr;

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use lapin::{BasicProperties, options::BasicPublishOptions};
use redis::AsyncCommands;
use tokio::time::sleep;

const GI_JOE_ID: i32 = 1;
const BARBIE_ID: i32 = 2;
const TEST_TIME: DateTime<Utc> = DateTime::from_timestamp_secs(0).unwrap();
const CUSTOMER1_ID: i32 = 101;
const CUSTOMER2_ID: i32 = 102;
const CUSTOMER3_ID: i32 = 103;
const CUSTOMER4_ID: i32 = 201;
const CUSTOMER5_ID: i32 = 202;
const CUSTOMER6_ID: i32 = 203;

async fn publish_toy_ordered(channel: &lapin::Channel, event: &ToyOrdered) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(event)?;

    channel
        .basic_publish(
            "",
            TOY_ORDERS_QUEUE,
            BasicPublishOptions::default(),
            &payload,
            BasicProperties::default(),
        )
        .await?
        .await?;

    Ok(())
}

async fn publish_toy_reviewed(channel: &lapin::Channel, event: &ToyReviewed) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(event)?;

    channel
        .basic_publish(
            "",
            TOY_REVIEWS_QUEUE,
            BasicPublishOptions::default(),
            &payload,
            BasicProperties::default(),
        )
        .await?
        .await?;

    Ok(())
}

async fn request_top_toys(
    api_address: SocketAddr,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> anyhow::Result<TopToysResponse> {
    Ok(reqwest::get(format!(
        "http://{}:{}/orders/top?start={}&end={}&count=2",
        api_address.ip(),
        api_address.port(),
        start.to_rfc3339_opts(SecondsFormat::Secs, true),
        end.to_rfc3339_opts(SecondsFormat::Secs, true)
    ))
    .await?
    .json()
    .await?)
}

async fn request_top_rated_toys(
    api_address: SocketAddr,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> anyhow::Result<TopRatedToysResponse> {
    Ok(reqwest::get(format!(
        "http://{}:{}/reviews/top_rated?start={}&end={}&count=2",
        api_address.ip(),
        api_address.port(),
        start.to_rfc3339_opts(SecondsFormat::Secs, true),
        end.to_rfc3339_opts(SecondsFormat::Secs, true)
    ))
    .await?
    .json()
    .await?)
}

/// Since we're using an async message queue and a database, we cannot assume that the data is immediately available after publishing an event.
/// This function utilize the worst solution for this issue - just wait a few seconds. For simplicity, we'll stick with this.
async fn wait_for_consistency() {
    tokio::time::sleep(Duration::seconds(1).to_std().unwrap()).await;
}

#[tokio::test]
async fn toy_order_received_top_toys_returns_top_results() -> anyhow::Result<()> {
    let env = TestEnv::start().await?;

    for event in [
        ToyOrdered::new(TEST_TIME, GI_JOE_ID, CUSTOMER1_ID),
        ToyOrdered::new(TEST_TIME + Duration::minutes(5), GI_JOE_ID, CUSTOMER2_ID),
        ToyOrdered::new(TEST_TIME + Duration::minutes(10), GI_JOE_ID, CUSTOMER3_ID),
        ToyOrdered::new(TEST_TIME + Duration::minutes(15), BARBIE_ID, CUSTOMER1_ID),
        ToyOrdered::new(TEST_TIME + Duration::minutes(20), BARBIE_ID, CUSTOMER2_ID),
    ] {
        publish_toy_ordered(&env.rabbitmq_channel, &event).await?;
    }

    wait_for_consistency().await;

    let top_response = request_top_toys(
        env.api_address,
        TEST_TIME + Duration::minutes(3),
        TEST_TIME + Duration::minutes(17),
    )
    .await?;

    assert_eq!(
        top_response,
        TopToysResponse {
            toys: vec![
                TopToyResponse {
                    toy_id: GI_JOE_ID,
                    orders: 2,
                },
                TopToyResponse {
                    toy_id: BARBIE_ID,
                    orders: 1,
                },
            ],
        }
    );

    Ok(())
}

#[tokio::test]
async fn toy_orders_and_reviews_received_top_rated_returns_highest_reviewed_toys()
-> anyhow::Result<()> {
    let mut env = TestEnv::start().await?;

    for event in [
        ToyOrdered::new(TEST_TIME, GI_JOE_ID, CUSTOMER1_ID),
        ToyOrdered::new(TEST_TIME, GI_JOE_ID, CUSTOMER2_ID),
        ToyOrdered::new(TEST_TIME, BARBIE_ID, CUSTOMER4_ID),
        ToyOrdered::new(TEST_TIME, BARBIE_ID, CUSTOMER5_ID),
        ToyOrdered::new(TEST_TIME, BARBIE_ID, CUSTOMER6_ID),
    ] {
        publish_toy_ordered(&env.rabbitmq_channel, &event).await?;
    }

    wait_for_consistency().await;

    for event in [
        ToyReviewed::new(TEST_TIME, GI_JOE_ID, CUSTOMER1_ID, 5),
        ToyReviewed::new(TEST_TIME + Duration::minutes(5), GI_JOE_ID, CUSTOMER2_ID, 4),
        ToyReviewed::new(
            TEST_TIME + Duration::minutes(10),
            BARBIE_ID,
            CUSTOMER4_ID,
            5,
        ),
        ToyReviewed::new(
            TEST_TIME + Duration::minutes(15),
            BARBIE_ID,
            CUSTOMER5_ID,
            5,
        ),
        ToyReviewed::new(
            TEST_TIME + Duration::minutes(20),
            BARBIE_ID,
            CUSTOMER6_ID,
            4,
        ),
    ] {
        publish_toy_reviewed(&env.rabbitmq_channel, &event).await?;
    }

    wait_for_consistency().await;

    let top_rated_response = request_top_rated_toys(
        env.api_address,
        TEST_TIME + Duration::minutes(3),
        TEST_TIME + Duration::minutes(17),
    )
    .await?;
    assert_eq!(
        top_rated_response,
        TopRatedToysResponse {
            toys: vec![
                crate::server::api_server::TopRatedToyResponse {
                    toy_id: BARBIE_ID,
                    average_score: 5.0,
                    reviews: 2,
                },
                crate::server::api_server::TopRatedToyResponse {
                    toy_id: GI_JOE_ID,
                    average_score: 4.0,
                    reviews: 1,
                },
            ],
        }
    );

    let cache_keys: Vec<String> = env.redis_conn.keys("http_cache:*").await?;
    assert_eq!(cache_keys.len(), 1);

    Ok(())
}

#[tokio::test]
async fn toy_orders_received_when_redis_is_down_results_are_returned() -> anyhow::Result<()> {
    let mut env = TestEnv::start().await?;

    sleep(Duration::seconds(2).to_std()?).await;

    for event in [
        ToyOrdered::new(TEST_TIME, 7, CUSTOMER1_ID),
        ToyOrdered::new(TEST_TIME + Duration::minutes(5), 7, CUSTOMER2_ID),
        ToyOrdered::new(TEST_TIME + Duration::minutes(10), 9, CUSTOMER4_ID),
    ] {
        publish_toy_ordered(&env.rabbitmq_channel, &event).await?;
    }

    env.stop_redis().await?;

    wait_for_consistency().await;

    let top_response = request_top_toys(
        env.api_address,
        TEST_TIME - Duration::minutes(1),
        TEST_TIME + Duration::minutes(30),
    )
    .await?;
    assert_eq!(
        top_response,
        TopToysResponse {
            toys: vec![
                crate::server::api_server::TopToyResponse {
                    toy_id: 7,
                    orders: 2,
                },
                crate::server::api_server::TopToyResponse {
                    toy_id: 9,
                    orders: 1,
                },
            ],
        }
    );

    publish_toy_ordered(
        &env.rabbitmq_channel,
        &ToyOrdered::new(TEST_TIME + Duration::minutes(15), 9, CUSTOMER5_ID),
    )
    .await?;

    wait_for_consistency().await;

    let updated_top_response = request_top_toys(
        env.api_address,
        TEST_TIME - Duration::minutes(1),
        TEST_TIME + Duration::minutes(30),
    )
    .await?;
    assert_eq!(
        updated_top_response,
        TopToysResponse {
            toys: vec![
                crate::server::api_server::TopToyResponse {
                    toy_id: 7,
                    orders: 2,
                },
                crate::server::api_server::TopToyResponse {
                    toy_id: 9,
                    orders: 2,
                },
            ],
        }
    );

    Ok(())
}
