use metric_aggregator::messages;
use rio_rs::cluster::storage::sql::SqlMembersStorage;
use rio_rs::prelude::*;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = SqlMembersStorage::pool()
        .max_connections(50)
        .connect("sqlite:///tmp/membership.sqlite3?mode=rwc")
        .await?;
    let members_storage = SqlMembersStorage::new(pool);

    sleep(Duration::from_secs(1)).await;

    members_storage.migrate().await;
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
            let _: messages::MetricResponse = client
                .send(
                    "MetricAggregator".to_string(),
                    instance_id.to_string(),
                    &payload,
                )
                .await?;
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
        let response: messages::MetricResponse = client
            .send(
                "MetricAggregator".to_string(),
                i.to_string(),
                &messages::GetMetric {},
            )
            .await?;
        println!("{} - {:?}", i, response);
    }
    Ok(())
}
