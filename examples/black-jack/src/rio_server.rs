use rio_rs::cluster::storage::sqlite::SqliteMembersStorage;
use rio_rs::object_placement::sqlite::SqliteObjectPlacementProvider;
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
        SqliteMembersStorage,
        PeerToPeerClusterProvider<SqliteMembersStorage>,
        SqliteObjectPlacementProvider,
    >,
> {
    let addr = format!("0.0.0.0:{port}");

    let registry = registry::server::registry();

    // Configure the Cluster Membership provider
    let pool = SqliteMembersStorage::pool()
        .connect(cluster_membership_provider_conn)
        .await?;
    let members_storage = SqliteMembersStorage::new(pool);

    let membership_provider_config = PeerToPeerClusterConfig::default();
    let membership_provider =
        PeerToPeerClusterProvider::new(members_storage, membership_provider_config);

    // Configure the object placement
    let pool = SqliteMembersStorage::pool()
        .connect(object_placement_provider_conn)
        .await?;

    let object_placement_provider = SqliteObjectPlacementProvider::new(pool);

    // Configure StateLoader + StateSaver
    let sql_state_pool = SqliteState::pool().connect(sql_state_conn).await?;
    let sql_state = SqliteState::new(sql_state_pool);
    // TODO StateLoader::prepare(&sql_state).await;
    StateSaver::prepare(&sql_state).await;

    // Create the server object
    let mut server = Server::new(
        addr,
        registry,
        membership_provider,
        object_placement_provider,
    );
    server.prepare().await;
    // LifecycleMessage will try to load object from state
    server.app_data(sql_state);

    Ok(server)
}
