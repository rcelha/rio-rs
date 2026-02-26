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

mod messages {
    use super::*;

    #[derive(TypeName, Message, Debug, Deserialize, Serialize)]
    pub struct Ping2 {}
}

mod services {
    use super::*;

    #[derive(Default, Debug, WithId, TypeName)]
    pub struct TestServicePingOnly {
        id: String,
    }

    impl ServiceObjectStateLoad for TestServicePingOnly {}
    impl ServiceObject for TestServicePingOnly {}

    #[async_trait]
    impl Handler<messages::Ping2> for TestServicePingOnly {
        type Returns = messages::Ping2;
        type Error = NoopError;

        async fn handle(
            &mut self,
            _message: messages::Ping2,
            _app_data: Arc<AppData>,
        ) -> Result<Self::Returns, Self::Error> {
            Ok(messages::Ping2 {})
        }
    }
}

make_registry! {
    TestService: [
        Ping => (Pong, NoopError),
        Pong => (Pong, NoopError),
    ],
    services::TestServicePingOnly: [
        messages::Ping2 => (messages::Ping2, NoopError),
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

    let _ping = client::test_service_ping_only::send_ping2(
        &mut client,
        "ping2",
        &messages::Ping2 {}
    );
    // todo async test
    Ok(())
}

fn main() {}
