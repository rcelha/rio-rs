use std::time::Duration;

use presence::messages::Ping;
use rio_rs::cluster::storage::sqlite::SqliteMembershipStorage;
use rio_rs::prelude::*;
use rio_rs::protocol::NoopError;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let pool = SqliteMembershipStorage::pool()
        .max_connections(50)
        .connect("sqlite:///tmp/presence-membership.sqlite3?mode=rwc")
        .await
        .unwrap();
    let members_storage = SqliteMembershipStorage::new(pool);
    sleep(Duration::from_secs(1)).await;

    members_storage.prepare().await;
    let servers = members_storage.active_members().await;
    println!("server: {:?}", servers);

    let mut client = ClientBuilder::new()
        .members_storage(members_storage)
        .build()
        .unwrap();

    let _resp: () = client
        .send::<_, NoopError>(
            "PresenceService",
            "player-1",
            &Ping {
                user_id: "player-1".to_string(),
            },
        )
        .await
        .unwrap();
}
