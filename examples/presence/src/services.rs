use std::sync::atomic::AtomicU32;
use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use rio_rs::server::{AdminCommands, AdminSender};
use rio_rs::state::local::LocalState;
use rio_rs::{app_data::AppDataExt, prelude::*};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::messages::Ping;

#[derive(Default, Debug, TypeName, Serialize, Deserialize)]
pub struct NoopState {}

#[derive(Default, Debug, TypeName, WithId, ManagedState)]
pub struct PresenceService {
    id: String,
    #[managed_state(provider = LocalState)]
    pub state: NoopState,
}

#[async_trait]
impl ServiceObject for PresenceService {
    async fn after_load(
        &mut self,
        app_data: Arc<AppData>,
    ) -> Result<(), ServiceObjectLifeCycleError> {
        let self_id = self.id().to_string();
        tokio::task::spawn(async move {
            let initial_count = app_data.get_or_default::<AtomicU32>();
            let initial_count = initial_count.load(std::sync::atomic::Ordering::Relaxed);

            loop {
                let global_counter = app_data.get_or_default::<AtomicU32>();
                if global_counter.load(std::sync::atomic::Ordering::Relaxed) - initial_count >= 3 {
                    break;
                }
                global_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                println!("tick: {:?}", global_counter);
                sleep(Duration::from_secs(1)).await;
            }

            let admin_sender = app_data.get::<AdminSender>();
            admin_sender
                .send(AdminCommands::Shutdown(
                    "PresenceService".to_string(),
                    self_id.clone(),
                ))
                .expect("TODO");
        });
        Ok(())
    }
}

#[async_trait]
impl Handler<Ping> for PresenceService {
    type Returns = ();
    async fn handle(
        &mut self,
        _message: Ping,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        let global_counter = app_data.get_or_default::<AtomicU32>();
        println!("Hello world {:?}", global_counter);
        Ok(())
    }
}
