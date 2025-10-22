use clap::Parser;
use rio_rs::cluster::storage::sqlite::SqliteMembershipStorage;
use rio_rs::object_placement::sqlite::SqliteObjectPlacement;
use rio_rs::prelude::*;
use rio_rs::state::sqlite::SqliteState;
use rio_rs::state::StateSaver;
use telemetry::messages;
use telemetry::services;

use once_cell::sync::Lazy;
use opentelemetry::global;
use opentelemetry_appender_tracing::layer;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
struct Args {
    #[clap(value_parser)]
    port: String,

    #[clap(short, value_parser)]
    membership_conn: Option<String>,

    #[clap(short, value_parser)]
    placement_conn: Option<String>,
}

static RESOURCE: Lazy<Resource> = Lazy::new(|| {
    Resource::builder()
        .with_service_name("basic-stdout-example")
        .build()
});

fn init_trace() -> SdkTracerProvider {
    let exporter = opentelemetry_stdout::SpanExporter::default();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter)
        .with_resource(RESOURCE.clone())
        .build();
    global::set_tracer_provider(provider.clone());
    provider
}

fn init_metrics() -> opentelemetry_sdk::metrics::SdkMeterProvider {
    let exporter = opentelemetry_stdout::MetricExporter::default();
    let provider = SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .with_resource(RESOURCE.clone())
        .build();
    global::set_meter_provider(provider.clone());
    provider
}

fn init_logs() -> opentelemetry_sdk::logs::SdkLoggerProvider {
    let filter_otel = EnvFilter::new("debug")
        .add_directive("sqlx=off".parse().unwrap())
        .add_directive("tokio=off".parse().unwrap());
    let exporter = opentelemetry_stdout::LogExporter::default();
    let provider: SdkLoggerProvider = SdkLoggerProvider::builder()
        .with_simple_exporter(exporter)
        .with_resource(RESOURCE.clone())
        .build();
    let layer = layer::OpenTelemetryTracingBridge::new(&provider).with_filter(filter_otel);
    tracing_subscriber::registry().with(layer).init();
    provider
}

#[tokio::main]
async fn main() {
    let _tracer_provider = init_trace();
    let _meter_provider = init_metrics();
    let _logger_provider = init_logs();

    let mut args = Args::parse();

    let addr = format!("0.0.0.0:{}", args.port);

    let members_storage_connection = args
        .membership_conn
        .get_or_insert("sqlite:///tmp/membership.sqlite3?mode=rwc".to_string());

    let placement_connection = args
        .placement_conn
        .get_or_insert("sqlite:///tmp/placement.sqlite3?mode=rwc".to_string());

    let mut registry = Registry::new();
    registry.add_type::<services::Room>();
    registry.add_handler::<services::Room, LifecycleMessage>();
    registry.add_handler::<services::Room, messages::Ping>();

    let num_cpus = std::thread::available_parallelism()
        .expect("error getting num of CPUs")
        .get() as u32;
    let num_cpus = num_cpus * 2;

    let pool = SqliteMembershipStorage::pool()
        .max_connections(num_cpus)
        .connect(members_storage_connection)
        .await
        .expect("Connection failure");
    let members_storage = SqliteMembershipStorage::new(pool);

    let mut cluster_config = PeerToPeerClusterConfig::default();
    cluster_config.interval_secs = 5;
    cluster_config.num_failures_threshold = 2;
    cluster_config.interval_secs_threshold = 30;
    let cluster = PeerToPeerClusterProvider::new(members_storage, cluster_config);

    let pool = SqliteObjectPlacement::pool()
        .max_connections(num_cpus)
        .connect(placement_connection)
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

    let sql_state_pool = SqliteState::pool()
        .max_connections(num_cpus)
        .connect("sqlite:///tmp/state.sqlite3?mode=rwc")
        .await
        .expect("TODO: Connection failure");
    let sql_state = SqliteState::new(sql_state_pool);
    StateSaver::<()>::prepare(&sql_state).await;
    server.app_data(sql_state);
    let listener = server.bind().await.unwrap();
    server.run(listener).await.unwrap();
}
