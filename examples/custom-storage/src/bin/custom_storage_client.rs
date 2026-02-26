use custom_storage::{messages, registry::client};
use rio_rs::cluster::storage::sqlite::SqliteMembershipStorage;
use rio_rs::prelude::*;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = SqliteMembershipStorage::pool()
        .max_connections(50)
        .connect("sqlite:///tmp/custom-storage-membership.sqlite3?mode=rwc")
        .await?;
    let members_storage = SqliteMembershipStorage::new(pool);

    sleep(Duration::from_secs(1)).await;

    members_storage.prepare().await;
    let servers = members_storage.active_members().await;
    println!("Servers: {:#?}", servers);

    let mut client = ClientBuilder::new()
        .members_storage(members_storage)
        .build()?;

    let resp = client::room::send_ping(
        &mut client,
        "1",
        &messages::Ping {
            ping_id: "1:1".to_string(),
        },
    )
    .await?;

    println!("Response: {:#?}", resp);
    Ok(())
}
