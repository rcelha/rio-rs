use metric_aggregator::registry::server::registry;
use metric_aggregator::services::Counter;
use rio_rs::cluster::storage::sqlite::SqliteMembershipStorage;
use rio_rs::object_placement::sqlite::SqliteObjectPlacement;
use rio_rs::state::sqlite::SqliteState;
use rio_rs::state::StateSaver;
use rio_rs::{prelude::*, state::local::LocalState};
use std::sync::atomic::AtomicUsize;

static USAGE: &str =
    "usage: server ip:port [MEMBERSHIP_CONNECTION_STRING] [PLACEMENT_CONNECTION_STRING]";

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let addr = args.next().expect(USAGE);
    let members_storage_connection = args
        .next()
        .unwrap_or("sqlite:///tmp/membership.sqlite3?mode=rwc".to_string());
    let placement_connection = args
        .next()
        .unwrap_or("sqlite:///tmp/placement.sqlite3?mode=rwc".to_string());

    let registry = registry();

    let num_cpus = std::thread::available_parallelism()
        .expect("error getting num of CPUs")
        .get() as u32;
    let num_cpus = num_cpus * 2;

    let pool = SqliteMembershipStorage::pool()
        .max_connections(num_cpus)
        .connect(&members_storage_connection)
        .await
        .expect("Connection failure");
    let members_storage = SqliteMembershipStorage::new(pool);

    let cluster = PeerToPeerClusterProvider::builder()
        .members_storage(members_storage)
        .build();

    let pool = SqliteObjectPlacement::pool()
        .max_connections(num_cpus)
        .connect(&placement_connection)
        .await
        .expect("Connection failure");

    let object_placement_provider = SqliteObjectPlacement::new(pool);

    let mut server = Server::builder()
        .address(addr.to_string())
        .registry(registry)
        .app_data(AppData::new())
        .cluster_provider(cluster)
        .object_placement_provider(object_placement_provider)
        .build();
    server.prepare().await;

    server.app_data(Counter(AtomicUsize::new(0)));
    server.app_data(LocalState::new());

    let sql_state_pool = SqliteState::pool()
        .connect("sqlite:///tmp/state.sqlite3?mode=rwc")
        .await
        .expect("Connection failure");
    let sql_state = SqliteState::new(sql_state_pool);
    StateSaver::<()>::prepare(&sql_state).await;
    server.app_data(sql_state);
    let listener = server.bind().await.unwrap();
    server.run(listener).await.expect("");
}
