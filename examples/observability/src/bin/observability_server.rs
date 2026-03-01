use clap::Parser;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::Protocol;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use tracing::Level;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use rio_rs::cluster::storage::sqlite::SqliteMembershipStorage;
use rio_rs::object_placement::sqlite::SqliteObjectPlacement;
use rio_rs::prelude::*;
use rio_rs::state::sqlite::SqliteState;
use rio_rs::state::StateSaver;

use observability::registry::server::registry;

#[derive(Parser, Debug)]
struct Args {
    #[clap(value_parser)]
    port: String,

    #[clap(short, value_parser)]
    membership_conn: Option<String>,

    #[clap(short, value_parser)]
    placement_conn: Option<String>,
}

#[tokio::main]
async fn main() {
    let mut args = Args::parse();

    let addr = format!("0.0.0.0:{}", args.port);

    // tracing
    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .build()
        .expect("OTLP Exporter");

    let resource = Resource::builder()
        .with_service_name("observability")
        .build();

    // Create a tracer provider with the exporter
    let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(otlp_exporter)
        .with_resource(resource)
        .build();

    // Set it as the global provider
    let tracer = tracer_provider.tracer("my_app_tracer");

    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            Level::INFO,
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer))
        .init();
    // end::tracing

    let members_storage_connection = args
        .membership_conn
        .get_or_insert("sqlite:///tmp/membership.sqlite3?mode=rwc".to_string());

    let placement_connection = args
        .placement_conn
        .get_or_insert("sqlite:///tmp/placement.sqlite3?mode=rwc".to_string());

    let registry = registry();

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

    let cluster = PeerToPeerClusterProvider::builder()
        .members_storage(members_storage)
        .build();

    let pool = SqliteObjectPlacement::pool()
        .max_connections(num_cpus)
        .connect(placement_connection)
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
    server.prepare().await.unwrap();

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
