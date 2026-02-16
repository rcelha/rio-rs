use futures::Future;
use rio_rs::state::local::LocalState;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::JoinSet;
use tokio::time::sleep;

use rio_rs::cluster::storage::local::LocalStorage;
use rio_rs::object_placement::local::LocalObjectPlacement;
use rio_rs::object_placement::ObjectPlacement;
use rio_rs::prelude::Registry;
use rio_rs::prelude::*;
use rio_rs::server::Server;

pub type LocalServer =
    Server<LocalStorage, PeerToPeerClusterProvider<LocalStorage>, LocalObjectPlacement>;

pub type BuildRegistry = dyn Fn() -> Registry;

async fn build_server(
    registry: Registry,
    members_storage: LocalStorage,
    object_placement_provider: LocalObjectPlacement,
) -> (LocalServer, TcpListener) {
    let membership_provider = PeerToPeerClusterProvider::builder()
        .members_storage(members_storage.clone())
        .interval_secs(1)
        .num_failures_threshold(1)
        .interval_secs_threshold(2)
        .drop_inactive_after_secs(3)
        .build();

    let mut server = Server::builder()
        .address("0.0.0.0:0".to_string())
        .registry(registry)
        .app_data(AppData::new())
        .cluster_provider(membership_provider)
        .object_placement_provider(object_placement_provider)
        .build();
    let listener = server.bind().await.expect("Bind Error");
    (server, listener)
}

#[allow(dead_code)]
/// Run a test and fail if it takes more then `timeout_seconds`
///
/// The test will run in a cluster of `num_servers` servers, all sharing the same
/// `members_storage` and `object_placement_provider`.
pub async fn run_integration_test<Fut>(
    timeout_seconds: u64,
    registry_builder: &BuildRegistry,
    members_storage: LocalStorage,
    object_placement_provider: LocalObjectPlacement,
    num_servers: usize,
    test_fn: impl FnOnce() -> Fut,
) where
    Fut: Future<Output = ()>,
{
    env_logger::try_init().ok();
    let mut servers = vec![];

    for _ in 0..num_servers {
        let registry = registry_builder();
        let (mut server, listener) = build_server(
            registry,
            members_storage.clone(),
            object_placement_provider.clone(),
        )
        .await;
        // TODO
        server.app_data(LocalState::default());
        servers.push((server, listener));
    }

    let test_fn_with_members = || async move {
        // Wait for cluster membership storage has some active servers
        while members_storage.active_members().await.unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        test_fn().await;
    };

    let mut tasks = JoinSet::new();
    for (mut server, listener) in servers.into_iter() {
        tasks.spawn(async move {
            let server_result = server.run(listener).await;
            drop(server);
            server_result
        });
    }

    tokio::select! {
        result = tasks.join_all() => {
            eprintln!("Server Result: {:?}", result);
            panic!("All servers have died");
        }
        _ = test_fn_with_members() => {}
        _ = tokio::time::sleep(Duration::from_secs(timeout_seconds)) => {
            panic!("Timeout reached");
        }
    };
}

#[allow(dead_code)]
/// Checks if an object is allocated to a server by looking it up in the object placement provider
pub async fn is_allocated(
    object_placement_provider: &impl ObjectPlacement,
    service_type: impl ToString,
    service_id: impl ToString,
) -> bool {
    let object_id = ObjectId(service_type.to_string(), service_id.to_string());
    let where_is_it = object_placement_provider.lookup(&object_id).await;
    where_is_it.is_some()
}

#[allow(dead_code)]
/// Polls until the number of active members matches the expected count.
/// Times out after `timeout` duration, polling every 100ms.
pub async fn wait_for_active_members<S: MembershipStorage>(
    members_storage: &S,
    expected_count: usize,
    timeout: Duration,
) {
    let start = std::time::Instant::now();
    loop {
        let active = members_storage.active_members().await.unwrap();
        if active.len() == expected_count {
            return;
        }
        if start.elapsed() > timeout {
            panic!(
                "Timeout waiting for {} active members, got {}",
                expected_count,
                active.len()
            );
        }
        sleep(Duration::from_millis(100)).await;
    }
}
