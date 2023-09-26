use std::{
    collections::HashSet,
    sync::Arc,
    thread::{self, JoinHandle},
};

use async_trait::async_trait;
use rio_rs::{prelude::*, state::sql::SqlState};
use serde::{Deserialize, Serialize};

use crate::{
    game_server::{
        build_app, AdminCommand, GameServerConfig, GameServerRequest, GameServerResponse,
    },
    messages,
};

static MAX_PLAYERS: usize = 3;

#[derive(Default, Debug, Serialize, Deserialize, TypeName)]
pub struct TableState {
    pub players: HashSet<String>,
}

#[derive(Default, Debug, TypeName, WithId, ManagedState)]
pub struct GameTable {
    pub id: String,
    #[managed_state(provider = SqlState)]
    pub state: Option<TableState>,

    // Game Server stuff
    pub thread_join_handler: Option<JoinHandle<()>>,
    pub response_rx: Option<crossbeam_channel::Receiver<GameServerResponse>>,
    pub request_tx: Option<crossbeam_channel::Sender<GameServerRequest>>,
}

impl GameTable {
    async fn save(&mut self, app_data: &AppData) {
        let state_saver = app_data.get::<SqlState>();
        self.save_state::<TableState, _>(state_saver)
            .await
            .expect("Cant save state");
    }

    fn start_game_server(&mut self) {
        let (command_tx, command_rx) = crossbeam_channel::unbounded();
        let (message_tx, message_rx) = crossbeam_channel::unbounded();

        let handler = thread::spawn(move || {
            let mut app = build_app(
                message_tx,
                command_rx,
                Some(GameServerConfig {
                    turn_duration_in_seconds: 10.0,
                }),
            );
            app.run();
        });
        self.thread_join_handler = Some(handler);
        self.response_rx = Some(message_rx);
        self.request_tx = Some(command_tx);
    }

    fn send_player_command(&mut self, command: GameServerRequest) -> GameServerResponse {
        if let (Some(tx), Some(rx)) = (&self.request_tx, &self.response_rx) {
            tx.send(command).expect("TODO");
            rx.recv().expect("TODO")
        } else {
            panic!("GameServer not initialized");
        }
    }
}

#[async_trait]
impl ServiceObject for GameTable {
    async fn after_load(&mut self, _: &AppData) -> Result<(), ServiceObjectLifeCycleError> {
        if self.state.is_none() {
            self.state = Some(Default::default())
        }

        self.start_game_server();
        Ok(())
    }

    async fn before_shutdown(&mut self, _: &AppData) -> Result<(), ServiceObjectLifeCycleError> {
        if let Some(tx) = self.request_tx.take() {
            tx.send(GameServerRequest::Admin(AdminCommand::Quit)).ok();
        }

        if let Some(handler) = self.thread_join_handler.take() {
            handler.join().ok();
        }
        Ok(())
    }
}

#[async_trait]
impl Handler<messages::JoinGame> for GameTable {
    type Returns = messages::JoinGameResponse;

    async fn handle(
        &mut self,
        message: messages::JoinGame,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        let state = self.state.as_mut().unwrap();
        let mut is_new_player = false;
        if state.players.len() < MAX_PLAYERS && !state.players.contains(&message.user_id) {
            state.players.insert(message.user_id.clone());
            is_new_player = true;
        }

        if is_new_player {
            self.send_player_command(GameServerRequest::Player(
                message.user_id,
                crate::game_server::PlayerCommand::Join,
            ));
            self.save(&app_data).await;
        }

        let state = self.state.as_mut().unwrap();
        let user_ids = state.players.iter().cloned().collect();

        Ok(messages::JoinGameResponse {
            table_id: self.id.clone(),
            user_ids,
        })
    }
}

#[async_trait]
impl Handler<messages::PlayerCommand> for GameTable {
    type Returns = messages::PlayerCommandResponse;

    async fn handle(
        &mut self,
        message: messages::PlayerCommand,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        let state = self.state.as_mut().unwrap();

        let resp = match message.0 {
            GameServerRequest::Player(player_id, player_command) => {
                if !state.players.contains(&player_id) {
                    GameServerResponse::Empty
                } else {
                    let inner_req = GameServerRequest::Player(player_id, player_command);
                    self.send_player_command(inner_req)
                }
            }
            _ => GameServerResponse::Empty,
        };

        self.save(&app_data).await;

        Ok(messages::PlayerCommandResponse(resp))
    }
}
