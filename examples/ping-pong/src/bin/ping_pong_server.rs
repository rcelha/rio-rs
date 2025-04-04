use clap::Parser;
use ping_pong::messages;
use ping_pong::services;
use rio_rs::cluster::storage::sqlite::SqliteMembershipStorage;
use rio_rs::object_placement::sqlite::SqliteObjectPlacement;
use rio_rs::prelude::*;
use rio_rs::state::sqlite::SqliteState;
use rio_rs::state::StateSaver;

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
