use presence::registry::server::registry;
use rio_rs::cluster::storage::sqlite::SqliteMembershipStorage;
use rio_rs::object_placement::sqlite::SqliteObjectPlacement;
use rio_rs::prelude::*;
use rio_rs::state::local::LocalState;

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
    SqliteMembershipStorage,
    PeerToPeerClusterProvider<SqliteMembershipStorage>,
    SqliteObjectPlacement,
> {
    let addr = format!("0.0.0.0:{port}");

    // Configure types on the server's registry
    let registry = registry();

    // Configure the Cluster Membership provider
    let pool = SqliteMembershipStorage::pool()
        .connect(cluster_membership_provider_conn)
        .await
        .unwrap();
    let members_storage = SqliteMembershipStorage::new(pool);

    let membership_provider = PeerToPeerClusterProvider::builder()
        .members_storage(members_storage)
        .build();

    // Configure the object placement
    let pool = SqliteMembershipStorage::pool()
        .connect(object_placement_provider_conn)
        .await
        .unwrap();

    let object_placement_provider = SqliteObjectPlacement::new(pool);

    // Create the server object
    let mut server = Server::builder()
        .address(addr)
        .registry(registry)
        .app_data(AppData::new())
        .cluster_provider(membership_provider)
        .object_placement_provider(object_placement_provider)
        .build();
    server.prepare().await;
    // LifecycleMessage will try to load object from state
    server.app_data(LocalState::default());
    server
}
