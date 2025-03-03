use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use log::debug;
use rio_rs::server::AdminSender;
use rio_rs::{object_placement::ObjectPlacementProvider, protocol::NoopError};
use serde::{Deserialize, Serialize};

use rio_rs::prelude::*;

use rio_rs::cluster::storage::local::LocalStorage;
use rio_rs::object_placement::local::LocalObjectPlacementProvider;

mod server_utils;
use server_utils::{is_allocated, run_integration_test};
use tokio::time::sleep;

#[derive(Default, Debug, Message, TypeName, Serialize, Deserialize)]
struct OkMessage {}

#[derive(Default, Debug, Message, TypeName, Serialize, Deserialize)]
struct KillServer {}

#[derive(Default, Debug, PartialEq, Message, TypeName, Serialize, Deserialize)]
struct MockResponse {}

#[derive(Debug, Default, WithId, TypeName, ManagedState)]
struct MockService {
    id: String,
}

#[async_trait]
impl Handler<OkMessage> for MockService {
    type Returns = MockResponse;
    type Error = ();
    async fn handle(
        &mut self,
        _message: OkMessage,
        _ctx: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error> {
        let resp = MockResponse {};
        Ok(resp)
    }
}

#[async_trait]
impl Handler<KillServer> for MockService {
    type Returns = MockResponse;
    type Error = ();
    async fn handle(
        &mut self,
        _message: KillServer,
        ctx: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error> {
        debug!("Let's try to kill it");
        let admin_interface = ctx.get::<AdminSender>();
        let _ = admin_interface
            .send(rio_rs::server::AdminCommands::ServerExit)
            .unwrap();
        debug!(".... Kill message sent");
        Ok(MockResponse {})
    }
}

fn build_registry() -> Registry {
    let mut registry = Registry::new();
    registry.add_type::<MockService>();
    registry.add_handler::<MockService, OkMessage>();
    registry.add_handler::<MockService, KillServer>();
    registry
}

// This test runs [move_object_on_server_failure_single] several times
// to ensure it will try to pick up the same server even after it gets
// terminated
//
// Unfortunetely this makes the test quite slow
#[tokio::test]
async fn move_object_on_server_failure() {
    for _ in 0..10 {
        move_object_on_server_failure_single().await;
    }
}

async fn move_object_on_server_failure_single() {
    let members_storage = LocalStorage::default();
    let object_placement_provider = LocalObjectPlacementProvider::default();

    run_integration_test(
        20,
        &build_registry,
        members_storage.clone(),
        object_placement_provider.clone(),
        2,
        || async move {
            let mut client = ClientBuilder::new()
                .members_storage(members_storage.clone())
                .build()
                .unwrap();

            let object_id = ObjectId::new("MockService", "1");
            // Starts with the object not allocated
            assert!(!is_allocated(&object_placement_provider, "MockService", "1").await);

            // As soon as we send the first message, the object is allocated to a server
            let message = OkMessage {};
            client
                .send::<MockResponse, NoopError>("MockService", "1", &message)
                .await
                .unwrap();
            assert!(is_allocated(&object_placement_provider, "MockService", "1").await);
            let first_server = object_placement_provider.lookup(&object_id).await.unwrap();

            // Now we send a message that will cause the server where this object is in to die.
            // Notice we need to wait a few seconds so the cluster provider marks it as inactive
            let message = KillServer {};
            client
                .send::<MockResponse, NoopError>("MockService", "1", &message)
                .await
                .unwrap();
            sleep(Duration::from_secs(5)).await;

            // The first server is now dead, we send another message to the object
            // so it gets re-allocated somewhere else in the cluster
            let message = OkMessage {};
            client
                .send::<MockResponse, NoopError>("MockService", "1", &message)
                .await
                .unwrap();
            // let members = members_storage.members().await.unwrap();
            assert!(is_allocated(&object_placement_provider, "MockService", "1").await);
            let second_server = object_placement_provider.lookup(&object_id).await.unwrap();

            // TODO why is this assert an _eq_ instead of a _ne_????
            assert_ne!(first_server, second_server);
        },
    )
    .await;
}
