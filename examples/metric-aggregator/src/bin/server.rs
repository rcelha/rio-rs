use metric_aggregator::{
    grains::{self, Counter},
    messages,
};
use rio_rs::prelude::*;
use rio_rs::{
    grain_placement_provider::sql::SqlGrainPlacementProvider,
    membership_provider::sql::SqlMembersStorage,
};
use std::sync::atomic::AtomicUsize;

static USAGE: &str =
    "usage: server ip:port [MEMBERSHIP_CONNECTION_STRING] [PLACEMENT_CONNECTION_STRING]";

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let addr = args.next().expect(USAGE);
    let members_storage_connection = args
        .next()
        .unwrap_or("sqlite:///tmp/membership.sqlite3".to_string());
    let placement_connection = args
        .next()
        .unwrap_or("sqlite:///tmp/placement.sqlite3".to_string());

    let mut registry = Registry::new();
    registry.add_static_fn::<grains::MetricAggregator, String, _>(FromId::from_id);
    registry.add_handler::<grains::MetricAggregator, LifecycleMessage>();
    registry.add_handler::<grains::MetricAggregator, messages::Ping>();
    registry.add_handler::<grains::MetricAggregator, messages::Metric>();
    registry.add_handler::<grains::MetricAggregator, messages::GetMetric>();

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

    let pool = SqlGrainPlacementProvider::pool()
        .max_connections(50)
        .connect(&placement_connection)
        .await
        .expect("Connection failure");

    let grain_placement_provider = SqlGrainPlacementProvider::new(pool);
    grain_placement_provider.migrate().await;

    let mut silo = Silo::new(
        addr.to_string(),
        registry,
        cluster,
        grain_placement_provider,
    );

    silo.app_data(Counter(AtomicUsize::new(0)));
    silo.serve().await;
}
