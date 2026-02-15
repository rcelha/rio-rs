use custom_storage::messages;
use custom_storage::ping_state::PingState;
use custom_storage::services;
use rio_rs::cluster::storage::sqlite::SqliteMembershipStorage;
use rio_rs::object_placement::sqlite::SqliteObjectPlacement;
use rio_rs::prelude::*;
use rio_rs::state::StateSaver;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port = "0";
    let addr = format!("0.0.0.0:{}", port);
    let members_storage_connection = "sqlite:///tmp/custom-storage-membership.sqlite3?mode=rwc";
    let placement_connection = "sqlite:///tmp/custom-storage-placement.sqlite3?mode=rwc";

    let mut registry = Registry::new();
    registry.add_type::<services::Room>();
    registry.add_handler::<services::Room, LifecycleMessage>();
    registry.add_handler::<services::Room, messages::Ping>();

    let pool = SqliteMembershipStorage::pool()
        .connect(members_storage_connection)
        .await?;
    let members_storage = SqliteMembershipStorage::new(pool);

    let cluster = PeerToPeerClusterProvider::builder()
        .members_storage(members_storage)
        .interval_secs(5)
        .num_failures_threshold(2)
        .interval_secs_threshold(30)
        .build();

    let pool = SqliteObjectPlacement::pool()
        .connect(placement_connection)
        .await?;
    let object_placement_provider = SqliteObjectPlacement::new(pool);

    let mut server = Server::builder()
        .address(addr.to_string())
        .registry(registry)
        .app_data(AppData::new())
        .cluster_provider(cluster)
        .object_placement_provider(object_placement_provider)
        .build();
    server.prepare().await;

    let state_pool = PingState::pool()
        .connect("sqlite:///tmp/state.sqlite3?mode=rwc")
        .await?;
    let ping_state = PingState::new(state_pool);
    ping_state.prepare().await;
    server.app_data(ping_state);
    let listener = server.bind().await.unwrap();
    server.run(listener).await.unwrap();
    Ok(())
}
