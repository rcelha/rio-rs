use futures::Future;
use std::time::Duration;

use rio_rs::cluster::storage::local::LocalStorage;
use rio_rs::object_placement::local::LocalObjectPlacementProvider;
use rio_rs::object_placement::ObjectPlacementProvider;
use rio_rs::prelude::Registry;
use rio_rs::prelude::*;
use rio_rs::server::Server;

pub type LocalServer =
    Server<LocalStorage, PeerToPeerClusterProvider<LocalStorage>, LocalObjectPlacementProvider>;

pub type BuildRegistry = dyn Fn() -> Registry;

#[allow(dead_code)] // It might be included on an integration test but not used
async fn build_server(
    registry: Registry,
    members_storage: LocalStorage,
    object_placement_provider: LocalObjectPlacementProvider,
) -> LocalServer {
    let mut cluster_provider_config = PeerToPeerClusterConfig::default();
    // Test connectivity every second. If, for the past 10 seconds, it had more than 1 failure, the
    // node will be marked as defective
    cluster_provider_config.interval_secs = 1;
    cluster_provider_config.num_failures_threshold = 3;
    cluster_provider_config.interval_secs_threshold = 10;
    let membership_provider =
        PeerToPeerClusterProvider::new(members_storage.clone(), cluster_provider_config);

    let mut server = Server::new(
        "0.0.0.0:0".to_string(),
        registry,
        membership_provider,
        object_placement_provider,
    );
    server.bind().await.expect("Bind Error");
    server
}

// Run a test and fail if it takes more then `timeout_seconds`
pub async fn run_integration_test<Fut>(
    timeout_seconds: u64,
    registry_builder: &BuildRegistry,
    members_storage: LocalStorage,
    object_placement_provider: LocalObjectPlacementProvider,
    num_servers: usize,
    test_fn: impl FnOnce() -> Fut,
) where
    Fut: Future<Output = ()>,
{
    let mut servers = vec![];

    for _ in 0..num_servers {
        let registry = registry_builder();
        let server = build_server(
            registry,
            members_storage.clone(),
            object_placement_provider.clone(),
        )
        .await;
        servers.push(server);
    }

    let test_fn_with_members = || async move {
        // Wait for cluster membership storage has some active servers
        while members_storage.active_members().await.unwrap().len() == 0 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        test_fn().await;
    };

    let mut tasks = vec![];
    for mut server in servers.into_iter() {
        let task = tokio::spawn(async move {
            let server_result = server.run().await;
            drop(server);
            server_result
        });
        tasks.push(task);
    }
    let servers_single_future = futures::future::join_all(tasks);

    tokio::select! {
        _ = servers_single_future => {
            panic!("All servers have died");
        }
        _ = test_fn_with_members() => {}
        _ = tokio::time::sleep(Duration::from_secs(timeout_seconds)) => {
            panic!("Timeout reached");
        }
    };
}

#[allow(dead_code)] // It might be included on an integration test but not used
pub async fn is_allocated(
    object_placement_provider: &impl ObjectPlacementProvider,
    service_type: impl ToString,
    service_id: impl ToString,
) -> bool {
    let object_id = ObjectId(service_type.to_string(), service_id.to_string());
    let where_is_it = object_placement_provider.lookup(&object_id).await;
    where_is_it.is_some()
}