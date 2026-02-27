use metric_aggregator::{messages, registry::client};
use rio_rs::cluster::storage::sqlite::SqliteMembershipStorage;
use rio_rs::prelude::*;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = SqliteMembershipStorage::pool()
        .max_connections(50)
        .connect("sqlite:///tmp/membership.sqlite3?mode=rwc")
        .await?;
    let members_storage = SqliteMembershipStorage::new(pool);

    sleep(Duration::from_secs(1)).await;

    members_storage.prepare().await;
    let servers = members_storage.active_members().await;
    println!("server: {:?}", servers);

    let mut client = ClientBuilder::new()
        .members_storage(members_storage)
        .build()?;

    let mut instance_map = HashMap::new();
    instance_map.insert("instance-1", "eu-west-1,EU");
    instance_map.insert("instance-2", "eu-east-1,EU");
    instance_map.insert("instance-3", "us-east-1,US");

    for (instance_id, tags) in instance_map {
        for i in 1..11 {
            let payload = messages::Metric {
                tags: tags.to_string(),
                value: 100 * i,
            };
            let _ =
                client::metric_aggregator::send_metric(&mut client, instance_id, &payload).await?;
            print!(".");
        }
    }
    println!("!");

    for i in [
        "instance-1",
        "instance-2",
        "eu-west-1",
        "us-east-1",
        "EU",
        "US",
    ] {
        let response =
            client::metric_aggregator::send_get_metric(&mut client, i, &messages::GetMetric {})
                .await?;
        println!("{} - {:?}", i, response);
    }
    Ok(())
}
