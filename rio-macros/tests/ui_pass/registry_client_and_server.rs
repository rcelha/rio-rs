use std::sync::Arc;

use async_trait::async_trait;
use rio_rs::cluster::storage::http::HttpMembershipStorage;
use rio_rs::prelude::*;
use serde::{Deserialize, Serialize};

use rio_macros::make_registry;

#[derive(Default, Debug, WithId, TypeName)]
struct TestService {
    id: String,
}

impl ServiceObjectStateLoad for TestService {}
impl ServiceObject for TestService {}

#[derive(TypeName, Message, Debug, Deserialize, Serialize)]
pub struct Ping {
    pub ping_id: String,
}

#[derive(TypeName, Message, Debug, Deserialize, Serialize)]
pub struct Pong {
    pub ping_id: String,
}

#[async_trait]
impl Handler<Ping> for TestService {
    type Returns = Pong;
    type Error = NoopError;

    async fn handle(
        &mut self,
        message: Ping,
        _app_data: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error> {
        Ok(Pong {
            ping_id: message.ping_id,
        })
    }
}

#[async_trait]
impl Handler<Pong> for TestService {
    type Returns = Pong;
    type Error = NoopError;

    async fn handle(
        &mut self,
        message: Pong,
        _app_data: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error> {
        Ok(Pong {
            ping_id: message.ping_id,
        })
    }
}

#[derive(Default, Debug, WithId, TypeName)]
struct TestServicePingOnly {
    id: String,
}

impl ServiceObjectStateLoad for TestServicePingOnly {}
impl ServiceObject for TestServicePingOnly {}

#[async_trait]
impl Handler<Ping> for TestServicePingOnly {
    type Returns = Ping;
    type Error = NoopError;

    async fn handle(
        &mut self,
        message: Ping,
        _app_data: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error> {
        Ok(Ping {
            ping_id: message.ping_id,
        })
    }
}

make_registry! {
    TestService: [
        Ping => (Pong, NoopError),
        Pong => (Pong, NoopError),
    ],
    TestServicePingOnly: [
        Ping => (Ping, NoopError),
    ]
}

async fn test_server() -> Result<(), Box<dyn std::error::Error>> {
    let _registry = server::registry();
    Ok(())
}

async fn test_client() -> Result<(), Box<dyn std::error::Error>> {
    // client usage
    let members_storage = HttpMembershipStorage {
        remote_address: "http://0.0.0.0:9876".to_string(),
    };
    let mut client = ClientBuilder::new()
        .members_storage(members_storage)
        .build()?;

    let _pong = client::test_service::send_ping(
        &mut client,
        "ping1",
        &Ping {
            ping_id: "ping1".to_string(),
        },
    )
    .await?;
    // let pong = client::test_service::send_pong;
    // let pong = client::test_service_ping_only::send_ping;
    // todo async test
    Ok(())
}

fn main() {}
