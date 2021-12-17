use super::errors::HandlerError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestEnvelope {
    pub handler_type: String,
    pub handler_id: String,
    pub message_type: String,
    pub payload: Vec<u8>,
}

impl RequestEnvelope {
    pub fn new(
        handler_type: String,
        handler_id: String,
        message_type: String,
        payload: Vec<u8>,
    ) -> RequestEnvelope {
        RequestEnvelope {
            handler_type,
            handler_id,
            message_type,
            payload,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub body: Result<Vec<u8>, String>,
}
impl ResponseEnvelope {
    pub fn new(body: Vec<u8>) -> ResponseEnvelope {
        ResponseEnvelope { body: Ok(body) }
    }
}

impl From<HandlerError> for ResponseEnvelope {
    fn from(error: HandlerError) -> Self {
        ResponseEnvelope {
            body: Err(error.to_string()),
        }
    }
}
