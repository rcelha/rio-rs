use async_trait::async_trait;
use rio_rs::prelude::*;
use rio_rs::protocol::NoopError;
use tracing::info;

use std::sync::Arc;

use super::*;

#[derive(Debug, Default, TypeName, WithId)]
pub struct Room {
    pub id: String,
}

impl ServiceObjectStateLoad for Room {}
impl ServiceObject for Room {}

#[async_trait]
impl Handler<messages::Ping> for Room {
    type Returns = messages::Pong;
    type Error = NoopError;

    async fn handle(
        &mut self,
        message: messages::Ping,
        _: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error> {
        info!("Ping Received");
        Ok(messages::Pong {
            ping_id: message.ping_id,
        })
    }
}
