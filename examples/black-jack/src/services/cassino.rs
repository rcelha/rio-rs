use std::sync::Arc;

use async_trait::async_trait;
use rio_rs::{cluster::storage::sql::SqlMembersStorage, prelude::*, state::sql::SqlState};
use serde::{Deserialize, Serialize};

use crate::messages;

#[derive(Default, Debug, Serialize, Deserialize, TypeName)]
pub struct CassinoState {
    pub table_ids: Vec<String>,
}

#[derive(Default, Debug, TypeName, FromId, ManagedState)]
pub struct Cassino {
    pub id: String,
    #[managed_state(provider = SqlState)]
    pub state: Option<CassinoState>,
}

impl Cassino {
    async fn save(&self, app_data: &AppData) {
        let state_saver = app_data.get::<SqlState>();
        self.save_state::<CassinoState, _>(state_saver)
            .await
            .expect("Cant save state for Cassino");
    }
}

#[async_trait]
impl ServiceObject for Cassino {
    async fn after_load(&mut self, _: &AppData) -> Result<(), ServiceObjectLifeCycleError> {
        if self.state.is_none() {
            self.state = Some(Default::default())
        }
        Ok(())
    }
}

#[async_trait]
impl Handler<messages::JoinGame> for Cassino {
    type Returns = messages::JoinGameResponse;

    async fn handle(
        &mut self,
        message: messages::JoinGame,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        if self.state.as_ref().unwrap().table_ids.len() == 0 {
            let new_uuid = uuid::Uuid::new_v4().to_string();
            self.state.as_mut().unwrap().table_ids.push(new_uuid);
            self.save(&app_data).await;
        }

        loop {
            let last_id = self.state.as_ref().unwrap().table_ids.last().unwrap();
            let table_response: messages::JoinGameResponse =
                Self::send::<SqlMembersStorage, _, _, _, _>(
                    &app_data,
                    &"GameTable",
                    last_id,
                    &message,
                )
                .await
                .unwrap();

            if table_response.user_ids.contains(&message.user_id) {
                return Ok(table_response);
            }
            let new_uuid = uuid::Uuid::new_v4().to_string();
            self.state.as_mut().unwrap().table_ids.push(new_uuid);
            self.save(&app_data).await;
        }
    }
}
