use metric_aggregator::services::{self, Counter};
use metric_aggregator::messages;
use rio_rs::cluster::storage::sql::SqlMembersStorage;
use rio_rs::object_placement::sql::SqlObjectPlacementProvider;
use rio_rs::state::sql::SqlState;
use rio_rs::{prelude::*, state::local::LocalState};
use sqlx::any::AnyPoolOptions;
use std::sync::atomic::AtomicUsize;

#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

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
    registry.add_static_fn::<services::MetricAggregator, String, _>(FromId::from_id);
    registry.add_handler::<services::MetricAggregator, LifecycleMessage>();
    registry.add_handler::<services::MetricAggregator, messages::Ping>();
    registry.add_handler::<services::MetricAggregator, messages::Metric>();
    registry.add_handler::<services::MetricAggregator, messages::GetMetric>();
    registry.add_handler::<services::MetricAggregator, messages::Drop>();

    let pool = SqlMembersStorage::pool()
        .max_connections(50)
        .connect(&members_storage_connection)
        .await
        .expect("Connection failure");
    let members_storage = SqlMembersStorage::new(pool);
    members_storage.migrate().await;

    let mut cluster_config = PeerToPeerClusterConfig::default();
    cluster_config.interval_secs = 5;
    cluster_config.num_failures_threshold = 2;
    cluster_config.interval_secs_threshold = 30;
    let cluster = PeerToPeerClusterProvider::new(members_storage, cluster_config);

    let pool = SqlObjectPlacementProvider::pool()
        .max_connections(50)
        .connect(&placement_connection)
        .await
        .expect("Connection failure");

    let object_placement_provider = SqlObjectPlacementProvider::new(pool);
    object_placement_provider.migrate().await;

    let mut silo = Server::new(
        addr.to_string(),
        registry,
        cluster,
        object_placement_provider,
    );

    silo.app_data(Counter(AtomicUsize::new(0)));
    silo.app_data(LocalState::new());

    // let sql_state = SqlState::new(AnyPoio)
    //
    let sql_state_pool = AnyPoolOptions::new()
        .max_connections(5)
        .connect("sqlite:///tmp/state.sqlite3?mode=rwc")
        .await
        .expect("TODO: Connection failure");
    let sql_state = SqlState::new(sql_state_pool);
    sql_state.migrate().await;
    silo.app_data(sql_state);

    silo.run().await;
}
