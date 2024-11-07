use rio_rs::cluster::storage::sql::SqlMembersStorage;
use rio_rs::object_placement::sql::SqlObjectPlacementProvider;
use rio_rs::prelude::*;
use rio_rs::state::sql::SqlState;
use rio_rs::state::StateSaver;

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
    registry.add_type::<Cassino>();
    registry.add_type::<GameTable>();
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

    let membership_provider_config = PeerToPeerClusterConfig::default();
    let membership_provider =
        PeerToPeerClusterProvider::new(members_storage, membership_provider_config);

    // Configure the object placement
    let pool = SqlMembersStorage::pool()
        .connect(object_placement_provider_conn)
        .await?;

    let object_placement_provider = SqlObjectPlacementProvider::new(pool);

    // Configure StateLoader + StateSaver
    let sql_state_pool = SqlState::pool().connect(sql_state_conn).await?;
    let sql_state = SqlState::new(sql_state_pool);
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
