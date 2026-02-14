use metric_aggregator::messages;
use metric_aggregator::services::{self, Counter};
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

    let mut registry = Registry::new();
    registry.add_type::<services::MetricAggregator>();
    registry.add_handler::<services::MetricAggregator, LifecycleMessage>();
    registry.add_handler::<services::MetricAggregator, messages::Ping>();
    registry.add_handler::<services::MetricAggregator, messages::Metric>();
    registry.add_handler::<services::MetricAggregator, messages::GetMetric>();
    registry.add_handler::<services::MetricAggregator, messages::Drop>();

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

    let cluster_config = PeerToPeerClusterConfig::new();
    let cluster = PeerToPeerClusterProvider::new(members_storage, cluster_config);

    let pool = SqliteObjectPlacement::pool()
        .max_connections(num_cpus)
        .connect(&placement_connection)
        .await
        .expect("Connection failure");

    let object_placement_provider = SqliteObjectPlacement::new(pool);

    let mut server = ServerBuilder::new()
        .address(addr.to_string())
        .registry(registry)
        .cluster_provider(cluster)
        .object_placement_provider(object_placement_provider)
        .client_pool_size(10)
        .build()
        .expect("TODO: server builder fail");
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
