use rio_rs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Message, Serialize, Deserialize, TypeName)]
pub struct Ping {
    pub user_id: String,
}
