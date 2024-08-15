use rio_rs::cluster::storage::sql::SqlMembersStorage;
use rio_rs::object_placement::sql::SqlObjectPlacementProvider;
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
    server.bind().await.unwrap();
    server.run().await.unwrap();
}

pub async fn build_server(
    port: &str,
    cluster_membership_provider_conn: &str,
    object_placement_provider_conn: &str,
) -> Server<
    SqlMembersStorage,
    PeerToPeerClusterProvider<SqlMembersStorage>,
    SqlObjectPlacementProvider,
> {
    let addr = format!("0.0.0.0:{port}");

    // Configure types on the server's registry
    let mut registry = Registry::new();
    registry.add_type::<PresenceService>();
    registry.add_handler::<PresenceService, LifecycleMessage>();
    registry.add_handler::<PresenceService, Ping>();

    // Configure the Cluster Membership provider
    let pool = SqlMembersStorage::pool()
        .connect(cluster_membership_provider_conn)
        .await
        .unwrap();
    let members_storage = SqlMembersStorage::new(pool);
    members_storage.migrate().await;

    let membership_provider_config = PeerToPeerClusterConfig::default();
    let membership_provider =
        PeerToPeerClusterProvider::new(members_storage, membership_provider_config);

    // Configure the object placement
    let pool = SqlMembersStorage::pool()
        .connect(object_placement_provider_conn)
        .await
        .unwrap();

    let object_placement_provider = SqlObjectPlacementProvider::new(pool);
    object_placement_provider.migrate().await;

    // Create the server object
    let mut server = Server::new(
        addr,
        registry,
        membership_provider,
        object_placement_provider,
    );
    // LifecycleMessage will try to load object from state
    server.app_data(LocalState::default());
    server
}
