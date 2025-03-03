use async_trait::async_trait;
use rio_rs::prelude::*;
use rio_rs::protocol::NoopError;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::*;

#[derive(Debug, Default, TypeName, WithId)]
pub struct Room {
    pub id: String,
    pub request_count: AtomicUsize,
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
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error> {
        let request_count = self.request_count.fetch_add(1, Ordering::Relaxed);
        if request_count >= 2 {
            self.shutdown(app_data).await.ok();
        }
        Ok(messages::Pong {
            ping_id: message.ping_id,
        })
    }
}
