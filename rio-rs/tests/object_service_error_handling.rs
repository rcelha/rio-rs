use std::sync::Arc;

use async_trait::async_trait;
use rio_rs::object_placement::local::LocalObjectPlacement;
use serde::{Deserialize, Serialize};

use rio_macros::{Message, TypeName, WithId};
use rio_rs::cluster::storage::local::LocalStorage;
use rio_rs::prelude::*;

mod server_utils;
use server_utils::{is_allocated, run_integration_test};
use thiserror::Error;

#[derive(Default, WithId, TypeName)]
struct MockService {
    id: String,
}

#[derive(Default, Debug, Message, TypeName, Serialize, Deserialize)]
struct OkMessage {}

#[derive(Default, Debug, Message, TypeName, Serialize, Deserialize)]
struct ErrorMessage {}

#[derive(Default, Debug, Message, TypeName, Serialize, Deserialize)]
struct PanicMessage {}

#[derive(Default, Debug, Message, TypeName, Serialize, Deserialize)]
struct MockResponse {}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
enum MockError {
    #[error("Mock")]
    Mock,
}

#[async_trait]
impl Handler<OkMessage> for MockService {
    type Returns = MockResponse;
    type Error = MockError;

    async fn handle(
        &mut self,
        _: OkMessage,
        _: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error> {
        let resp = MockResponse {};
        Ok(resp)
    }
}

#[async_trait]
impl Handler<ErrorMessage> for MockService {
    type Returns = MockResponse;
    type Error = MockError;
    async fn handle(
        &mut self,
        _: ErrorMessage,
        _: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error> {
        Err(MockError::Mock)
    }
}

#[async_trait]
impl Handler<PanicMessage> for MockService {
    type Returns = MockResponse;
    type Error = MockError;

    async fn handle(
        &mut self,
        _: PanicMessage,
        _: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error> {
        panic!("Error handling message");
    }
}

fn build_registry() -> Registry {
    let mut registry = Registry::new();
    registry.add_type::<MockService>();
    registry.add_handler::<MockService, OkMessage>();
    registry.add_handler::<MockService, ErrorMessage>();
    registry.add_handler::<MockService, PanicMessage>();
    registry
}

#[tokio::test]
async fn service_is_allocated_ok() {
    let members_storage = LocalStorage::default();
    let object_placement_provider = LocalObjectPlacement::default();

    run_integration_test(
        20,
        &build_registry,
        members_storage.clone(),
        object_placement_provider.clone(),
        1,
        || async move {
            assert!(!is_allocated(&object_placement_provider, "MockService", "1").await);
            let mut client = ClientBuilder::new()
                .members_storage(members_storage)
                .build()
                .unwrap();
            let message = OkMessage {};
            let resp: Result<MockResponse, RequestError<MockError>> =
                client.send("MockService", "1", &message).await;
            assert!(resp.is_ok());
            assert!(is_allocated(&object_placement_provider, "MockService", "1").await);
        },
    )
    .await;
}

#[tokio::test]
async fn service_is_allocated_after_error() {
    let members_storage = LocalStorage::default();
    let object_placement_provider = LocalObjectPlacement::default();

    run_integration_test(
        20,
        &build_registry,
        members_storage.clone(),
        object_placement_provider.clone(),
        1,
        || async move {
            let mut client = ClientBuilder::new()
                .members_storage(members_storage)
                .build()
                .unwrap();

            assert!(!is_allocated(&object_placement_provider, "MockService", "1").await);

            let message = ErrorMessage {};
            let resp: Result<MockResponse, RequestError<MockError>> =
                client.send("MockService", "1", &message).await;
            assert!(resp.is_err());
            assert!(is_allocated(&object_placement_provider, "MockService", "1").await);
        },
    )
    .await;
}

#[tokio::test]
async fn service_is_not_allocated_after_panic() {
    let members_storage = LocalStorage::default();
    let object_placement_provider = LocalObjectPlacement::default();

    run_integration_test(
        20,
        &build_registry,
        members_storage.clone(),
        object_placement_provider.clone(),
        1,
        || async move {
            let mut client = ClientBuilder::new()
                .members_storage(members_storage)
                .build()
                .unwrap();
            assert!(!is_allocated(&object_placement_provider, "MockService", "1").await);

            let message = PanicMessage {};
            let resp: Result<MockResponse, RequestError<MockError>> =
                client.send("MockService", "1", &message).await;
            assert!(resp.is_err());
            assert!(!is_allocated(&object_placement_provider, "MockService", "1").await);
        },
    )
    .await;
}

// TODO test for panic on pubsub?
