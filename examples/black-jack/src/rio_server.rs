use rio_rs::cluster::storage::sql::SqlMembersStorage;
use rio_rs::object_placement::sql::SqlObjectPlacementProvider;
use rio_rs::prelude::*;
use rio_rs::state::sql::SqlState;

use crate::messages::{JoinGame, PlayerCommand};
use crate::services::cassino::Cassino;
use crate::services::table::GameTable;

pub async fn build_server(
    port: &str,
    cluster_membership_provider_conn: &str,
    object_placement_provider_conn: &str,
    sql_state_conn: &str,
) -> anyhow::Result<
    Server<
        SqlMembersStorage,
        PeerToPeerClusterProvider<SqlMembersStorage>,
        SqlObjectPlacementProvider,
    >,
> {
    let addr = format!("0.0.0.0:{port}");

    // Configure types on the server's registry
    let mut registry = Registry::new();
    registry.add_static_fn::<Cassino, String, _>(FromId::from_id);
    registry.add_static_fn::<GameTable, String, _>(FromId::from_id);
    registry.add_handler::<Cassino, LifecycleMessage>();
    registry.add_handler::<Cassino, JoinGame>();
    registry.add_handler::<GameTable, LifecycleMessage>();
    registry.add_handler::<GameTable, JoinGame>();
    registry.add_handler::<GameTable, PlayerCommand>();

    // Configure the Cluster Membership provider
    let pool = SqlMembersStorage::pool()
        .connect(cluster_membership_provider_conn)
        .await?;
    let members_storage = SqlMembersStorage::new(pool);
    members_storage.migrate().await;

    let membership_provider_config = PeerToPeerClusterConfig::default();
    let membership_provider =
        PeerToPeerClusterProvider::new(members_storage, membership_provider_config);

    // Configure the object placement
    let pool = SqlMembersStorage::pool()
        .connect(object_placement_provider_conn)
        .await?;

    let object_placement_provider = SqlObjectPlacementProvider::new(pool);
    object_placement_provider.migrate().await;

    // Configure StateLoader + StateSaver
    let sql_state_pool = SqlState::pool().connect(sql_state_conn).await?;
    let sql_state = SqlState::new(sql_state_pool);
    sql_state.migrate().await;

    // Create the server object
    let mut server = Server::new(
        addr,
        registry,
        membership_provider,
        object_placement_provider,
    );
    // LifecycleMessage will try to load object from state
    server.app_data(sql_state);

    Ok(server)
}
