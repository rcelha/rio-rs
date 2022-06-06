use super::errors::HandlerError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    pub body: Result<Vec<u8>, ResponseError>,
}
impl ResponseEnvelope {
    pub fn new(body: Vec<u8>) -> ResponseEnvelope {
        ResponseEnvelope { body: Ok(body) }
    }

    pub fn err(error: ResponseError) -> ResponseEnvelope {
        ResponseEnvelope { body: Err(error) }
    }
}

impl From<HandlerError> for ResponseEnvelope {
    fn from(error: HandlerError) -> Self {
        ResponseEnvelope {
            body: Err(ResponseError::Unknown(error.to_string())),
        }
    }
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum ResponseError {
    #[error("ServiceObject is in another server")]
    Redirect(String),

    #[error("ServiceObject had to be deallocated")]
    DeallocateServiceObject,

    #[error("unknown execution error")]
    Unknown(String),

    #[error("handler error")]
    HandlerError(String),
}

impl From<HandlerError> for ResponseError {
    fn from(error: HandlerError) -> Self {
        ResponseError::HandlerError(error.to_string())
    }
}
