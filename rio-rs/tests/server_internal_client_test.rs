use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use rio_rs::prelude::*;

use rio_rs::cluster::storage::local::LocalStorage;
use rio_rs::object_placement::local::LocalObjectPlacementProvider;
use rio_rs::state::local::LocalState;

mod server_utils;
use server_utils::run_integration_test;
use thiserror::Error;

#[derive(Debug, Default, TypeName, Serialize, Deserialize)]
struct MockState {
    value: String,
}

#[derive(Debug, Default, WithId, TypeName, ManagedState)]
struct MockService {
    id: String,
    #[managed_state(provider = LocalState)]
    mock_state: MockState,
}

#[derive(Default, Debug, Clone, Message, TypeName, Serialize, Deserialize)]
struct MockMessage {
    text: String,
    send_to: Option<String>,
}

#[derive(Default, Debug, Message, TypeName, Serialize, Deserialize)]
struct MockResponse {
    text: String,
}

#[derive(Debug, Serialize, Deserialize, Error, Clone)]
enum MockError {
    #[error("Upstream error")]
    UpstreamError(String),
}

#[async_trait]
impl Handler<MockMessage> for MockService {
    type Returns = MockResponse;
    type Error = MockError;
    async fn handle(
        &mut self,
        message: MockMessage,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error> {
        let resp = if let Some(to) = &message.send_to {
            let mut msg = message.clone();
            msg.send_to = None;
            let resp = Self::send(&app_data, "MockService", to, &msg)
                .await
                .map_err(|err: RequestError<MockError>| {
                    MockError::UpstreamError(err.to_string())
                })?;
            resp
        } else {
            MockResponse {
                text: format!("{} received {}", self.id, message.text),
            }
        };
        Ok(resp)
    }
}

#[async_trait]
impl ServiceObject for MockService {}

fn build_registry() -> Registry {
    let mut registry = Registry::new();
    registry.add_type::<MockService>();
    registry.add_handler::<MockService, MockMessage>();
    registry.add_handler::<MockService, LifecycleMessage>();
    registry
}

#[tokio::test]
async fn request_response_with_proxy() {
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
                send_to: Some("2000".to_string()),
            };
            let resp: MockResponse = client
                .send::<_, MockError>("MockService", "1", &message)
                .await
                .unwrap();
            assert_eq!(&resp.text, "2000 received hi");
        },
    )
    .await;
}
