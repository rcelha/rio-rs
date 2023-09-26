use async_trait::async_trait;
use futures::Future;
use rio_macros::{Message, TypeName, WithId};

use rio_rs::{prelude::*, HandleSubscription};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use rio_rs::cluster::storage::LocalStorage;
use rio_rs::object_placement::local::LocalObjectPlacementProvider;

#[derive(Default, WithId, TypeName)]
struct MockService {
    id: String,
}

#[derive(Default, Debug, Message, TypeName, Serialize, Deserialize)]
struct MockMessage {
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
impl Handler<HandleSubscription> for MockService {
    type Returns = ();
    async fn handle(
        &mut self,
        _message: HandleSubscription,
        _: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        Ok(())
    }
}

type LocalServer =
    Server<LocalStorage, PeerToPeerClusterProvider<LocalStorage>, LocalObjectPlacementProvider>;

async fn build_server(members_storage: LocalStorage) -> LocalServer {
    let mut registry = Registry::new();
    registry.add_type::<MockService>();
    registry.add_handler::<MockService, MockMessage>();
    registry.add_handler::<MockService, HandleSubscription>();

    let membership_provider =
        PeerToPeerClusterProvider::new(members_storage.clone(), Default::default());
    let object_placement_provider = LocalObjectPlacementProvider::default();
    Server::new(
        "0.0.0.0:44444".into(),
        registry,
        membership_provider,
        object_placement_provider,
    )
}

async fn run_integration_test<Fut>(
    timeout_seconds: u64,
    members_storage: LocalStorage,
    test_fn: impl FnOnce() -> Fut,
) where
    Fut: Future<Output = ()>,
{
    let mut server = build_server(members_storage.clone()).await;

    let test_fn_with_members = || async move {
        while members_storage.active_members().await.unwrap().len() == 0 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        test_fn().await;
    };

    tokio::select! {
        server = server.run() => {
            server.unwrap();
        }
        _ = test_fn_with_members() => {}
        _ = tokio::time::sleep(Duration::from_secs(timeout_seconds)) => {
            panic!("Timeout reached");
        }
    };
}

#[ignore]
#[tokio::test]
async fn test_request_response() {
    let members_storage = LocalStorage::default();
    run_integration_test(5, members_storage.clone(), || async move {
        let mut client = ClientBuilder::new()
            .members_storage(members_storage)
            .build()
            .unwrap();
        let message = MockMessage {
            text: "hi".to_string(),
        };
        let resp: MockResponse = client.send("MockService", "1", &message).await.unwrap();
        assert_eq!(&resp.text, "1 received hi");
    })
    .await;
}

// TODO this will always fail, as client.subscribe doesn't return an async iter yet
// When the implementation is good, it should no longer panic
#[should_panic]
#[tokio::test]
async fn test_pubsub() {
    let members_storage = LocalStorage::default();
    run_integration_test(5, members_storage.clone(), || async move {
        let mut client = ClientBuilder::new()
            .members_storage(members_storage)
            .build()
            .unwrap();
        let _resp = client.subscribe("MockService", "1").await;
    })
    .await;
}
