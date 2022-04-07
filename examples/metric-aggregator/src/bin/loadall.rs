use metric_aggregator::messages;
use rio_rs::{membership_provider::sql::SqlMembersStorage, prelude::*};
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
    let silos = members_storage.active_members().await;
    println!("server: {:?}", silos);

    let mut client = ClientBuilder::new()
        .members_storage(members_storage)
        .build()?;

    for i in 1..20_000 {
        let payload = messages::Metric {
            tags: "".to_string(),
            value: 100 * i,
        };
        let _: messages::MetricResponse = client
            .send(
                "MetricAggregator".to_string(),
                format!("instace-{}", i),
                &payload,
            )
            .await?;
        print!(".");
    }
    println!("!");
    Ok(())
}
