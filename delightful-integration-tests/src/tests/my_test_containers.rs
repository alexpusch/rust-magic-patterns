#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use bollard::{
        Docker,
        query_parameters::{CreateContainerOptions, RemoveContainerOptions, StartContainerOptions},
        secret::ContainerCreateBody,
    };

    use crate::tests::async_drop::async_drop;

    struct RabbitMqContainer {
        docker: Docker,
        container_id: String,
    }

    impl RabbitMqContainer {
        async fn start() -> anyhow::Result<Self> {
            let docker = Docker::connect_with_local_defaults()?;

            let container = docker
                .create_container(
                    Option::<CreateContainerOptions>::None,
                    ContainerCreateBody {
                        image: Some("rabbitmq:3.8.22-management".to_string()),
                        exposed_ports: Some(HashMap::from([(
                            "5672/tcp".to_string(),
                            HashMap::new(),
                        )])),
                        ..Default::default()
                    },
                )
                .await?;

            docker
                .start_container(&container.id, Some(StartContainerOptions::default()))
                .await?;

            Ok(Self {
                docker,
                container_id: container.id,
            })
        }
    }

    impl Drop for RabbitMqContainer {
        fn drop(&mut self) {
            println!("Dropping container");

            let docker = self.docker.clone();
            let container_id = self.container_id.clone();

            async_drop(async move {
                docker
                    .remove_container(
                        &container_id,
                        Some(RemoveContainerOptions {
                            force: true,
                            ..Default::default()
                        }),
                    )
                    .await
                    .expect("Failed to remove container");
            });
        }
    }

    #[tokio::test]
    async fn rabbitmq_container_test_example() -> anyhow::Result<()> {
        let _rabbitmq = RabbitMqContainer::start().await?;

        // Use RabbitMQ to drive your integration tests.
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        Ok(())
    }
}
