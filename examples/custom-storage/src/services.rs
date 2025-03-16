use async_trait::async_trait;
use rio_rs::prelude::*;
use rio_rs::protocol::NoopError;

use std::sync::Arc;

use crate::ping_state::{PingAttributeState, PingState};

use super::*;

#[derive(Debug, Default, TypeName, WithId, ManagedState)]
pub struct Room {
    pub id: String,
    #[managed_state(provider = PingState)]
    pub state: PingAttributeState,
}

#[async_trait]
impl ServiceObject for Room {
    async fn before_load(
        &mut self,
        _app_data: Arc<AppData>,
    ) -> Result<(), ServiceObjectLifeCycleError> {
        println!("I will load my state {}", self.id);
        println!("And my count is {:?}", self.state);
        Ok(())
    }

    async fn after_load(
        &mut self,
        _app_data: Arc<AppData>,
    ) -> Result<(), ServiceObjectLifeCycleError> {
        println!("I have loaded {}", self.id);
        println!("And my count is {:?}", self.state);
        Ok(())
    }
}

#[async_trait]
impl Handler<messages::Ping> for Room {
    type Returns = messages::Pong;
    type Error = NoopError;

    async fn handle(
        &mut self,
        message: messages::Ping,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error> {
        self.state.request_count += 1;
        let state_saver = app_data.get::<PingState>();
        self.save_state(state_saver).await.expect("TODO");
        Ok(messages::Pong {
            ping_id: message.ping_id,
            request_count: self.state.request_count,
        })
    }
}
