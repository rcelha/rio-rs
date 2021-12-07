use super::*;
use serde::{Deserialize, Serialize};

pub struct Human {}
impl IdentifiableType for Human {
    fn user_defined_type_id() -> &'static str {
        "Human"
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HiMessage {
    pub name: String,
}
impl IdentifiableType for HiMessage {
    fn user_defined_type_id() -> &'static str {
        "HiMessage"
    }
}
impl Message for HiMessage {
    type Returns = Self;
}

#[derive(Serialize, Deserialize)]
pub struct GoodbyeMessage {
    pub a: (),
}

impl IdentifiableType for GoodbyeMessage {
    fn user_defined_type_id() -> &'static str {
        "GoodbyeMessage"
    }
}
impl Message for GoodbyeMessage {
    type Returns = String;
}

impl Handler<HiMessage> for Human {
    fn handle(&mut self, _message: HiMessage) -> Result<HiMessage, HandlerError> {
        Ok(HiMessage {
            name: "uai".to_string(),
        })
    }
}
impl Handler<GoodbyeMessage> for Human {
    fn handle(&mut self, _message: GoodbyeMessage) -> Result<String, HandlerError> {
        Ok("bye".to_string())
    }
}
