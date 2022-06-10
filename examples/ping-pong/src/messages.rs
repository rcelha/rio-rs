use rio_rs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(TypeName, Message, Debug, Deserialize, Serialize)]
pub struct Ping {
    pub ping_id: String,
}

#[derive(TypeName, Message, Debug, Deserialize, Serialize)]
pub struct Pong {
    pub ping_id: String,
}
