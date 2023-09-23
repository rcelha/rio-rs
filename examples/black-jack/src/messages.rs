use rio_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::game_server::{GameServerRequest, GameServerResponse};

#[derive(Debug, TypeName, Serialize, Deserialize, Message)]
pub struct JoinGame {
    pub user_id: String,
}

#[derive(Debug, TypeName, Serialize, Deserialize, Message)]
pub struct JoinGameResponse {
    pub table_id: String,
    pub user_ids: Vec<String>,
}

#[derive(Debug, TypeName, Serialize, Deserialize, Message)]
pub struct PlayerCommand(pub GameServerRequest);

#[derive(Debug, TypeName, Serialize, Deserialize, Message)]
pub struct PlayerCommandResponse(pub GameServerResponse);
