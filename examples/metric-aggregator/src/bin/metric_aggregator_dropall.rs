use metric_aggregator::{messages, registry::client};
use rio_rs::cluster::storage::sqlite::SqliteMembershipStorage;
use rio_rs::prelude::*;
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

    for i in 1..20_000 {
        client::metric_aggregator::send_drop(
            &mut client,
            &format!("instace-{}", i),
            &messages::Drop {},
        )
        .await?;
        print!(".");
    }
    println!("!");
    Ok(())
}
