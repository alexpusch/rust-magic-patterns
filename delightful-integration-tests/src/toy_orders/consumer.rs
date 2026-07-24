use futures::StreamExt;
use lapin::{
    Channel, Consumer,
    options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions, QueueDeclareOptions},
    types::FieldTable,
};
use tracing::{error, info};

use crate::{events::ToyOrdered, toy_orders::ToyOrdersService};

pub const TOY_ORDERS_QUEUE: &str = "toy_orders";

pub struct ToyOrdersConsumer {
    toy_orders_service: ToyOrdersService,
    consumer: Consumer,
}

impl ToyOrdersConsumer {
    pub async fn new(
        channel: Channel,
        toy_orders_service: ToyOrdersService,
    ) -> anyhow::Result<Self> {
        let queue = channel
            .queue_declare(
                TOY_ORDERS_QUEUE,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;
        info!(?queue, "Declared queue");

        let consumer = channel
            .basic_consume(
                TOY_ORDERS_QUEUE,
                "toy_orders_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        info!("Toy orders consumer started");

        Ok(Self {
            toy_orders_service,
            consumer,
        })
    }

    pub async fn consume(self) -> anyhow::Result<()> {
        let mut consumer = self.consumer;

        while let Some(delivery) = consumer.next().await {
            let delivery = match delivery {
                Ok(delivery) => delivery,
                Err(e) => {
                    error!("Error receiving toy order message: {:?}", e);
                    continue;
                }
            };

            match serde_json::from_slice::<ToyOrdered>(&delivery.data) {
                Ok(msg) => {
                    info!("Received toy order: {:?}", msg);

                    let result = self.toy_orders_service.handle_toy_order(msg).await;

                    match result {
                        Ok(_) => delivery.ack(BasicAckOptions::default()).await?,
                        Err(e) => {
                            error!("Failed to insert toy order: {:?}", e);
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
                        "Failed to deserialize toy order message: {:?}. Data: {:?}",
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
