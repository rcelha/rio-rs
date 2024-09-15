use metric_aggregator::messages;
use metric_aggregator::services::{self, Counter};
use rio_rs::cluster::storage::sql::SqlMembersStorage;
use rio_rs::object_placement::sql::SqlObjectPlacementProvider;
use rio_rs::state::sql::SqlState;
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

    let pool = SqlMembersStorage::pool()
        .max_connections(num_cpus)
        .connect(&members_storage_connection)
        .await
        .expect("Connection failure");
    let members_storage = SqlMembersStorage::new(pool);

    let mut cluster_config = PeerToPeerClusterConfig::default();
    cluster_config.interval_secs = 5;
    cluster_config.num_failures_threshold = 2;
    cluster_config.interval_secs_threshold = 30;
    let cluster = PeerToPeerClusterProvider::new(members_storage, cluster_config);

    let pool = SqlObjectPlacementProvider::pool()
        .max_connections(num_cpus)
        .connect(&placement_connection)
        .await
        .expect("Connection failure");

    let object_placement_provider = SqlObjectPlacementProvider::new(pool);

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

    let sql_state_pool = SqlState::pool()
        .connect("sqlite:///tmp/state.sqlite3?mode=rwc")
        .await
        .expect("Connection failure");
    let sql_state = SqlState::new(sql_state_pool);
    sql_state.prepare().await;
    server.app_data(sql_state);
    server.bind().await.unwrap();
    server.run().await.expect("");
}
