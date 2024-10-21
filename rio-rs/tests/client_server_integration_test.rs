use async_trait::async_trait;
use futures::{pin_mut, StreamExt};
use rio_macros::{Message, TypeName, WithId};

use rio_rs::app_data::AppDataExt;
use rio_rs::cluster::storage::local::LocalStorage;
use rio_rs::message_router::MessageRouter;
use rio_rs::object_placement::local::LocalObjectPlacementProvider;
use rio_rs::prelude::*;
use rio_rs::protocol::pubsub::SubscriptionResponse;
use rio_rs::registry::IdentifiableType;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

mod server_utils;
use server_utils::run_integration_test;

#[derive(Default, WithId, TypeName)]
struct MockService {
    id: String,
}

#[derive(Default, Debug, Message, TypeName, Serialize, Deserialize)]
struct MockMessage {
    text: String,
}

#[derive(Default, Debug, Message, TypeName, Serialize, Deserialize)]
struct CausePublishMessage {
    text: String,
}

#[derive(Default, Debug, Message, TypeName, Serialize, Deserialize)]
struct MockResponse {
    text: String,
}

#[async_trait]
impl Handler<MockMessage> for MockService {
    type Returns = MockResponse;
    async fn handle(
        &mut self,
        message: MockMessage,
        _: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        let resp = MockResponse {
            text: format!("{} received {}", self.id, message.text),
        };
        Ok(resp)
    }
}

#[async_trait]
impl Handler<CausePublishMessage> for MockService {
    type Returns = ();
    async fn handle(
        &mut self,
        message: CausePublishMessage,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        let router = app_data.get_or_default::<MessageRouter>();
        let type_id = MockService::user_defined_type_id();
        let byte_message = bincode::serialize(&message).unwrap();
        let packed_message = SubscriptionResponse {
            body: Ok(byte_message),
        };
        router.publish(type_id.to_string(), self.id.clone(), packed_message);
        Ok(())
    }
}

fn build_registry() -> Registry {
    let mut registry = Registry::new();
    registry.add_type::<MockService>();
    registry.add_handler::<MockService, MockMessage>();
    registry.add_handler::<MockService, CausePublishMessage>();
    registry
}

#[tokio::test]
async fn request_response() {
    let members_storage = LocalStorage::default();
    let object_placement_provider = LocalObjectPlacementProvider::default();
    run_integration_test(
        5,
        &build_registry,
        members_storage.clone(),
        object_placement_provider.clone(),
        1,
        || async move {
            let mut client = ClientBuilder::new()
                .members_storage(members_storage)
                .build()
                .unwrap();
            let message = MockMessage {
                text: "hi".to_string(),
            };
            let resp: MockResponse = client.send("MockService", "1", &message).await.unwrap();
            assert_eq!(&resp.text, "1 received hi");
        },
    )
    .await;
}

#[tokio::test]
async fn request_response_redirectt() {
    let members_storage = LocalStorage::default();
    let object_placement_provider = LocalObjectPlacementProvider::default();

    run_integration_test(
        5,
        &build_registry,
        members_storage.clone(),
        object_placement_provider.clone(),
        10,
        || async move {
            let mut client = ClientBuilder::new()
                .members_storage(members_storage)
                .build()
                .unwrap();
            let message = MockMessage {
                text: "hi".to_string(),
            };
            let resp: MockResponse = client.send("MockService", "1", &message).await.unwrap();
            assert_eq!(&resp.text, "1 received hi");
        },
    )
    .await;
}

#[tokio::test]
async fn pubsub() {
    let members_storage = LocalStorage::default();
    let object_placement_provider = LocalObjectPlacementProvider::default();

    run_integration_test(
        5,
        &build_registry,
        members_storage.clone(),
        object_placement_provider.clone(),
        1,
        || async move {
            let another_members_storage = members_storage.clone();
            let publishing_task = tokio::spawn(async move {
                let mut client = ClientBuilder::new()
                    .members_storage(another_members_storage)
                    .build()
                    .unwrap();

                for i in 0..10 {
                    let _: () = client
                        .send(
                            "MockService",
                            "1",
                            &CausePublishMessage {
                                text: format!("hey {}", i),
                            },
                        )
                        .await
                        .unwrap();
                }
            });
            let mut client = ClientBuilder::new()
                .members_storage(members_storage)
                .build()
                .unwrap();

            let resp = client
                .subscribe::<CausePublishMessage>("MockService", "1")
                .await;

            pin_mut!(resp);

            let mut recv_count = 0;
            // it will timeout if it doesn't get the last message
            while let Some(Ok(i)) = resp.next().await {
                recv_count += 1;
                if i.text == "hey 9" {
                    break;
                }
            }
            assert!(recv_count > 0);
            publishing_task.abort();
        },
    )
    .await;
}

#[tokio::test]
async fn pubsub_redirect() {
    let members_storage = LocalStorage::default();
    let object_placement_provider = LocalObjectPlacementProvider::default();

    run_integration_test(
        15,
        &build_registry,
        members_storage.clone(),
        object_placement_provider.clone(),
        10,
        || async move {
            let another_members_storage = members_storage.clone();

            let publishing_task = tokio::spawn(async move {
                let mut client = ClientBuilder::new()
                    .members_storage(another_members_storage)
                    .build()
                    .unwrap();

                for i in 0..100 {
                    let _: () = client
                        .send(
                            "MockService",
                            "1",
                            &CausePublishMessage {
                                text: format!("hey {}", i),
                            },
                        )
                        .await
                        .expect("Send Error");
                }
            });

            let mut client = ClientBuilder::new()
                .members_storage(members_storage)
                .build()
                .expect("Client Build Error");

            // Hack to try to cause a redirect
            // This test can have false positives if there is no redirect
            sleep(Duration::from_millis(10)).await;
            let subscription = client
                .subscribe::<CausePublishMessage>("MockService", "1")
                .await;
            pin_mut!(subscription);

            // it will timeout if it doesn't get the last message
            let mut recv_count = 0;
            while let Some(message_result) = subscription.next().await {
                let i = if let Ok(message) = message_result {
                    message
                } else {
                    break;
                };
                recv_count += 1;
                if i.text == "hey 99" {
                    break;
                }
            }
            assert!(recv_count > 0);
            publishing_task.abort();
        },
    )
    .await;
}
