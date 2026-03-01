use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use rio_rs::prelude::*;

use rio_rs::cluster::storage::local::LocalStorage;
use rio_rs::object_placement::local::LocalObjectPlacement;
use rio_rs::state::local::LocalState;

mod server_utils;
use server_utils::{is_allocated, run_integration_test};
use thiserror::Error;

#[derive(Default, Debug, Message, TypeName, Serialize, Deserialize)]
struct OkMessage {}

#[derive(Default, Debug, PartialEq, Message, TypeName, Serialize, Deserialize)]
struct MockResponse {}

#[derive(Debug, Default, TypeName, Serialize, Deserialize)]
struct MockState {
    value: String,
}

#[derive(Debug, Serialize, Deserialize, Error, Clone)]
enum MockError {}

#[derive(Debug, Default, WithId, TypeName, ManagedState)]
struct MockService {
    id: String,
    #[managed_state(provider = LocalState)]
    state: MockState,
}

#[async_trait]
impl ServiceObject for MockService {
    async fn before_load(&mut self, _: Arc<AppData>) -> Result<(), ServiceObjectLifeCycleError> {
        if &self.id == "1" {
            panic!("Panico");
        } else if &self.id == "2" {
            Err(ServiceObjectLifeCycleError::Unknown)
        } else {
            Ok(())
        }
    }
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

fn build_registry() -> Registry {
    let mut registry = Registry::new();
    registry.add_type::<MockService>();
    registry.add_handler::<MockService, LifecycleMessage>();
    registry
}

#[tokio::test]
async fn service_is_not_allocated_on_lifecycle_handlers_panic() {
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

            let message = OkMessage {};
            let resp: Result<MockResponse, RequestError<MockError>> =
                client.send("MockService", "1", &message).await;
            assert!(matches!(
                resp,
                Err(RequestError::ResponseError(ResponseError::Allocate))
            ));
            assert!(!is_allocated(&object_placement_provider, "MockService", "1").await);
        },
    )
    .await;
}

#[tokio::test]
async fn service_is_not_allocated_on_lifecycle_handlers_error() {
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
            assert!(!is_allocated(&object_placement_provider, "MockService", "2").await);

            let message = OkMessage {};
            let resp: Result<MockResponse, RequestError<MockError>> =
                client.send("MockService", "1", &message).await;
            assert!(matches!(
                resp,
                Err(RequestError::ResponseError(ResponseError::Allocate))
            ));
            assert!(!is_allocated(&object_placement_provider, "MockService", "2").await);
        },
    )
    .await;
}
