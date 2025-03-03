use std::{
    collections::HashSet,
    sync::Arc,
    thread::{self, JoinHandle},
};

use async_trait::async_trait;
use rio_rs::{
    app_data::AppDataExt,
    message_router::MessageRouter,
    prelude::*,
    protocol::{pubsub::SubscriptionResponse, NoopError},
    registry::IdentifiableType,
    state::sqlite::SqliteState,
};
use serde::{Deserialize, Serialize};

use crate::{
    game_server::{
        build_app, AdminCommand, GameServerConfig, GameServerRequest, GameServerResponse,
    },
    messages::{self, PlayerPush},
};

static MAX_PLAYERS: usize = 3;

#[derive(Default, Debug, Serialize, Deserialize, TypeName)]
pub struct TableState {
    pub players: HashSet<String>,
}

#[derive(Default, Debug, TypeName, WithId, ManagedState)]
pub struct GameTable {
    pub id: String,
    #[managed_state(provider = SqliteState)]
    pub state: TableState,

    // Game Server stuff
    pub thread_join_handler: Option<JoinHandle<()>>,
    pub msg_receiver_join_handler: Option<JoinHandle<()>>,
    pub response_rx: Option<crossbeam_channel::Receiver<GameServerResponse>>,
    pub request_tx: Option<crossbeam_channel::Sender<GameServerRequest>>,
}

impl GameTable {
    async fn save(&mut self, app_data: &AppData) {
        let state_saver = app_data.get::<SqliteState>();
        self.save_state::<TableState, _>(state_saver)
            .await
            .expect("Cant save state");
    }

    fn start_game_server(&mut self, app_data: Arc<AppData>) {
        let (command_tx, command_rx) = crossbeam_channel::unbounded();
        let (message_tx, message_rx) = crossbeam_channel::unbounded();
        let (changes_tx, changes_rx) = crossbeam_channel::unbounded();

        let handler = thread::spawn(move || {
            let mut app = build_app(
                message_tx,
                command_rx,
                changes_tx,
                Some(GameServerConfig {
                    turn_duration_in_seconds: 10.0,
                }),
            );
            app.run();
        });

        let inner_app_data = app_data.clone();
        let my_id = self.id.clone();
        let msg_receiver_join_handler = thread::spawn(move || {
            while let Ok(i) = changes_rx.recv() {
                let router = inner_app_data.get_or_default::<MessageRouter>();
                let type_id = Self::user_defined_type_id();
                let message = PlayerPush(i);
                let byte_message = bincode::serialize(&message).unwrap();
                let packed_message = SubscriptionResponse {
                    body: Ok(byte_message),
                };
                router.publish(type_id.to_string(), my_id.clone(), packed_message);
            }
        });

        self.thread_join_handler = Some(handler);
        self.response_rx = Some(message_rx);
        self.request_tx = Some(command_tx);
        self.msg_receiver_join_handler = Some(msg_receiver_join_handler);
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
    async fn after_load(
        &mut self,
        app_data: Arc<AppData>,
    ) -> Result<(), ServiceObjectLifeCycleError> {
        self.load(&app_data).await.ok();
        self.start_game_server(app_data);
        Ok(())
    }

    async fn before_shutdown(
        &mut self,
        _: Arc<AppData>,
    ) -> Result<(), ServiceObjectLifeCycleError> {
        if let Some(tx) = self.request_tx.take() {
            tx.send(GameServerRequest::Admin(AdminCommand::Quit)).ok();
        }

        if let Some(handler) = self.thread_join_handler.take() {
            handler.join().ok();
        }

        if let Some(handler) = self.msg_receiver_join_handler.take() {
            handler.join().ok();
        }

        Ok(())
    }
}

#[async_trait]
impl Handler<messages::JoinGame> for GameTable {
    type Returns = messages::JoinGameResponse;
    type Error = NoopError;

    async fn handle(
        &mut self,
        message: messages::JoinGame,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error> {
        let mut is_new_player = false;
        if self.state.players.len() < MAX_PLAYERS && !self.state.players.contains(&message.user_id)
        {
            self.state.players.insert(message.user_id.clone());
            is_new_player = true;
        }

        if is_new_player {
            self.send_player_command(GameServerRequest::Player(
                message.user_id,
                crate::game_server::PlayerCommand::Join,
            ));
            self.save(&app_data).await;
        }

        let user_ids = self.state.players.iter().cloned().collect();

        Ok(messages::JoinGameResponse {
            table_id: self.id.clone(),
            user_ids,
        })
    }
}

#[async_trait]
impl Handler<messages::PlayerCommand> for GameTable {
    type Returns = messages::PlayerCommandResponse;
    type Error = NoopError;

    async fn handle(
        &mut self,
        message: messages::PlayerCommand,
        app_data: Arc<AppData>,
    ) -> Result<Self::Returns, Self::Error> {
        let resp = match message.0 {
            GameServerRequest::Player(player_id, player_command) => {
                if !self.state.players.contains(&player_id) {
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
