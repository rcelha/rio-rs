use async_trait::async_trait;
use rio_rs::prelude::*;
use rio_rs::state::sql::SqlState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::task::JoinHandle;

use super::*;

#[derive(Default, Debug, Clone, Serialize, Deserialize, TypeName)]
pub struct InnerState {
    pub count_by_service: Arc<RwLock<HashMap<String, i32>>>,
}

#[derive(Debug, Default, TypeName, FromId, ManagedState)]
pub struct Room {
    pub id: String,
    task: Option<JoinHandle<()>>,
    #[managed_state(provider = SqlState)]
    inner_state: Option<InnerState>,
}

async fn room_loop(inner_state: InnerState) {
    loop {
        println!("tick {:?}", inner_state);
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

#[async_trait]
impl ServiceObject for Room {
    async fn after_load(&mut self, _: &AppData) -> Result<(), ServiceObjectLifeCycleError> {
        if self.inner_state.is_none() {
            self.inner_state = Some(Default::default())
        }

        let inner_state = self.inner_state.as_ref().expect("TODO").clone();
        self.task = Some(tokio::task::spawn(async move {
            room_loop(inner_state).await;
        }));

        Ok(())
    }

    async fn before_shutdown(
        &mut self,
        app_data: &AppData,
    ) -> Result<(), ServiceObjectLifeCycleError> {
        let state_saver = app_data.get::<SqlState>();
        self.save_state::<InnerState, _>(state_saver)
            .await
            .expect("TODO save_state");

        if let Some(task) = self.task.take() {
            task.abort();
            task.await
                .map_err(|e| {
                    println!("There was an error finishing the task {:?}", e);
                })
                .ok();
        }
        println!("I am shutdown");
        Ok(())
    }
}

#[async_trait]
impl Handler<messages::Ping> for Room {
    type Returns = messages::Pong;
    async fn handle(
        &mut self,
        message: messages::Ping,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        let e = {
            let mut state_guard = self
                .inner_state
                .as_mut()
                .expect("TODO as_mut")
                .count_by_service
                .write()
                .expect("TODO LOCK");
            let e = state_guard.entry(self.id.clone()).or_default();
            *e += 1;
            e.clone()
        };
        println!("e {}", e);
        if e >= 3 {
            self.shutdown(&app_data).await.expect("TODO shutdown");
        }
        Ok(messages::Pong {
            ping_id: message.ping_id,
        })
    }
}
