use futures::StreamExt;
use lapin::{
    Channel, Consumer,
    options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions, QueueDeclareOptions},
    types::FieldTable,
};
use tracing::{error, info};

use crate::{events::ToyReviewed, toy_reviews::ToyReviewsService};

pub const TOY_REVIEWS_QUEUE: &str = "toy_reviews";

pub struct ToyReviewConsumer {
    toy_reviews_service: ToyReviewsService,
    consumer: Consumer,
}

impl ToyReviewConsumer {
    pub async fn new(
        channel: Channel,
        toy_reviews_service: ToyReviewsService,
    ) -> anyhow::Result<Self> {
        let queue = channel
            .queue_declare(
                TOY_REVIEWS_QUEUE,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;
        info!(?queue, "Declared queue");

        let consumer = channel
            .basic_consume(
                TOY_REVIEWS_QUEUE,
                "toy_reviews_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok(Self {
            toy_reviews_service,
            consumer,
        })
    }

    pub async fn consume(self) -> anyhow::Result<()> {
        info!("Toy reviews consumer started");
        let mut consumer = self.consumer;

        while let Some(delivery) = consumer.next().await {
            let delivery = match delivery {
                Ok(delivery) => delivery,
                Err(e) => {
                    error!("Error receiving toy review message: {:?}", e);
                    continue;
                }
            };

            match serde_json::from_slice::<ToyReviewed>(&delivery.data) {
                Ok(msg) => {
                    info!("Received toy review: {:?}", msg);

                    let result = self.toy_reviews_service.handle_toy_review(msg).await;

                    match result {
                        Ok(()) => delivery.ack(BasicAckOptions::default()).await?,
                        Err(error) => {
                            error!("Failed to insert toy review: {:?}", error);

                            delivery
                                .nack(BasicNackOptions {
                                    requeue: false,
                                    ..Default::default()
                                })
                                .await?
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to deserialize toy review message: {:?}. Data: {:?}",
                        e,
                        String::from_utf8_lossy(&delivery.data)
                    );
                    delivery
                        .nack(BasicNackOptions {
                            requeue: false,
                            ..Default::default()
                        })
                        .await?
                }
            }
        }

        Ok(())
    }
}
