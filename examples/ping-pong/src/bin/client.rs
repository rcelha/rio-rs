use ping_pong::messages;
use rio_rs::cluster::storage::sql::SqlMembersStorage;
use rio_rs::prelude::*;
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

    members_storage.prepare().await;
    let servers = members_storage.active_members().await;
    println!("server: {:?}", servers);

    let mut client = ClientBuilder::new()
        .members_storage(members_storage)
        .build()?;

    let resp: messages::Pong = client
        .send(
            "Room".to_string(),
            "1".to_string(),
            &messages::Ping {
                ping_id: "1:1".to_string(),
            },
        )
        .await
        .unwrap();

    println!("Response: {:#?}", resp);
    Ok(())
}
