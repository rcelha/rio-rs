use std::time::SystemTime;

use futures::FutureExt;
use metric_aggregator::messages;
use rand::{thread_rng, Rng};
use rio_rs::client::ClientConnectionManager;
use rio_rs::cluster::storage::sql::SqlMembersStorage;
use rio_rs::prelude::*;

static USAGE: &str = "usage: load_client PARALLEL_REQUEST NUM_CLIENTS [NUM_REQUESTS=1000] [NUM_IDS=1000] [DB_CONN_STRING=sqlite:///tmp/membership.sqlite3?mode=rwc]";

#[derive(Debug, Clone)]
struct Options {
    db_conn: String,
    parallel_requests: usize,
    num_clients: usize,
    num_requests: usize,
    num_ids: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let parse = |x: String| x.parse().ok();
    let options = Options {
        parallel_requests: args.next().and_then(parse).expect(USAGE),
        num_clients: args.next().and_then(parse).expect(USAGE),
        num_requests: args.next().and_then(parse).unwrap_or(1000),
        num_ids: args.next().and_then(parse).unwrap_or(1000),
        db_conn: args
            .next()
            .unwrap_or("sqlite:///tmp/membership.sqlite3?mode=rwc".to_string()),
    };
    println!("{:#?}", options);
    let rt = tokio::runtime::Builder::new_multi_thread()
        //.worker_threads(options.num_clients)
        .enable_all()
        .build()?;

    let t0 = SystemTime::now();
    rt.block_on(async {
        let tasks = (0..options.num_clients).map(|i| {
            println!("Starting task #{}", i);
            let opts = options.clone();
            rt.spawn(async move {
                let err_msg = format!("Error with task #{}", i);
                client(opts).await.expect(&err_msg);
                println!("Finished task #{}", i);
            })
        });
        futures::future::join_all(tasks).await;
    });

    let time_elapsed = t0.elapsed().unwrap().as_secs_f64();
    let total_requests = options.num_clients * options.num_requests * options.parallel_requests;
    let reqs = total_requests as f64 / time_elapsed;

    println!("Requests: {}", total_requests);
    println!("Time:     {}s", time_elapsed);
    println!("Req/s:    {}", reqs);

    rt.shutdown_background();

    Ok(())
}

async fn client(opts: Options) -> Result<(), Box<dyn std::error::Error>> {
    let pool = SqlMembersStorage::pool()
        .max_connections(opts.parallel_requests as u32)
        .connect(&opts.db_conn)
        .await?;
    let members_storage = SqlMembersStorage::new(pool);

    members_storage
        .members()
        .map(|x| println!("Members={:#?}", x))
        .await;

    let conn_manager = ClientBuilder::new()
        .members_storage(members_storage)
        .build_connection_manager()?;
    let client_pool = ClientConnectionManager::pool()
        .max_size(opts.parallel_requests as u32)
        .build(conn_manager)
        .await?;

    let parallel_tasks = (0..opts.parallel_requests).map(|_| async {
        let client_pool = client_pool.clone();
        let opts = opts.clone();

        tokio::spawn(async move {
            for _ in 0..opts.num_requests {
                let mut client = client_pool.get().await.unwrap();
                let object_id = { thread_rng().gen_range(0..opts.num_ids).to_string() };
                let resp: messages::Pong = client
                    .send(
                        "MetricAggregator".to_string(),
                        object_id.clone(),
                        &messages::Ping {
                            ping_id: object_id.clone(),
                        },
                    )
                    .await
                    .unwrap();
                if resp.ping_id != object_id {
                    panic!("{} != {}", resp.ping_id, object_id);
                }
            }
        })
        .await
    });

    futures::future::join_all(parallel_tasks).await;

    Ok(())
}
