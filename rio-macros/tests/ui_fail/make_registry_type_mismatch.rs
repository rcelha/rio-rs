// TODO create ui_fail for this
//
use std::sync::Arc;

use async_trait::async_trait;
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

make_registry! {
    TestService: [
        Ping => (Ping, NoopError),
    ]
}

fn main() {
    let _registry = server::registry();
}
