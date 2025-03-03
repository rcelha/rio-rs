use rio_rs::cluster::storage::sqlite::SqliteMembersStorage;
use rio_rs::object_placement::sqlite::SqliteObjectPlacementProvider;
use rio_rs::prelude::*;
use rio_rs::state::local::LocalState;

use presence::messages::Ping;
use presence::services::PresenceService;

#[tokio::main]
async fn main() {
    let mut server = build_server(
        "8888",
        "sqlite:///tmp/presence-membership.sqlite3?mode=rwc",
        "sqlite:///tmp/presence-placement.sqlite3?mode=rwc",
    )
    .await;
    let listener = server.bind().await.unwrap();
    server.run(listener).await.unwrap();
}

pub async fn build_server(
    port: &str,
    cluster_membership_provider_conn: &str,
    object_placement_provider_conn: &str,
) -> Server<
    SqliteMembersStorage,
    PeerToPeerClusterProvider<SqliteMembersStorage>,
    SqliteObjectPlacementProvider,
> {
    let addr = format!("0.0.0.0:{port}");

    // Configure types on the server's registry
    let mut registry = Registry::new();
    registry.add_type::<PresenceService>();
    registry.add_handler::<PresenceService, LifecycleMessage>();
    registry.add_handler::<PresenceService, Ping>();

    // Configure the Cluster Membership provider
    let pool = SqliteMembersStorage::pool()
        .connect(cluster_membership_provider_conn)
        .await
        .unwrap();
    let members_storage = SqliteMembersStorage::new(pool);

    let membership_provider_config = PeerToPeerClusterConfig::default();
    let membership_provider =
        PeerToPeerClusterProvider::new(members_storage, membership_provider_config);

    // Configure the object placement
    let pool = SqliteMembersStorage::pool()
        .connect(object_placement_provider_conn)
        .await
        .unwrap();

    let object_placement_provider = SqliteObjectPlacementProvider::new(pool);

    // Create the server object
    let mut server = Server::new(
        addr,
        registry,
        membership_provider,
        object_placement_provider,
    );
    server.prepare().await;
    // LifecycleMessage will try to load object from state
    server.app_data(LocalState::default());
    server
}
