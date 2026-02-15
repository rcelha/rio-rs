use rio_rs::cluster::storage::sqlite::SqliteMembershipStorage;
use rio_rs::object_placement::sqlite::SqliteObjectPlacement;
use rio_rs::prelude::*;
use rio_rs::state::sqlite::SqliteState;
use rio_rs::state::StateSaver;

use crate::registry;

pub async fn build_server(
    port: &str,
    cluster_membership_provider_conn: &str,
    object_placement_provider_conn: &str,
    sql_state_conn: &str,
) -> anyhow::Result<
    Server<
        SqliteMembershipStorage,
        PeerToPeerClusterProvider<SqliteMembershipStorage>,
        SqliteObjectPlacement,
    >,
> {
    let addr = format!("0.0.0.0:{port}");

    let registry = registry::server::registry();

    // Configure the Cluster Membership provider
    let pool = SqliteMembershipStorage::pool()
        .connect(cluster_membership_provider_conn)
        .await?;
    let members_storage = SqliteMembershipStorage::new(pool);

    let membership_provider_config = PeerToPeerClusterConfig::default();
    let membership_provider =
        PeerToPeerClusterProvider::new(members_storage, membership_provider_config);

    // Configure the object placement
    let pool = SqliteMembershipStorage::pool()
        .connect(object_placement_provider_conn)
        .await?;

    let object_placement_provider = SqliteObjectPlacement::new(pool);

    // Configure StateLoader + StateSaver
    let sql_state_pool = SqliteState::pool().connect(sql_state_conn).await?;
    let sql_state = SqliteState::new(sql_state_pool);
    // TODO StateLoader::prepare(&sql_state).await;
    StateSaver::<()>::prepare(&sql_state).await;

    // Create the server object
    let mut server = Server::builder()
        .address(addr)
        .app_data(AppData::new())
        .http_members_storage_address("0.0.0.0:9876".to_string())
        .registry(registry)
        .cluster_provider(membership_provider)
        .object_placement_provider(object_placement_provider)
        .build();
    server.prepare().await;
    // LifecycleMessage will try to load object from state
    server.app_data(sql_state);

    Ok(server)
}
