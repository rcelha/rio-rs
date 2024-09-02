//! Client/Server communication protocol

use super::errors::HandlerError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// This is the struct that we serialize and send to the server serialized
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// This is the struct that we serialize and send back to the client
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

/// Error that we serialize back inside of the [ResponseEnvelope]
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum ResponseError {
    #[error("ServiceObject is in another server")]
    Redirect(String),

    #[error("ServiceObject had to be deallocated")]
    DeallocateServiceObject,

    #[error("unknown execution error")]
    Unknown(String),

    #[error("handler error")]
    HandlerError(String),

    #[error("error deserializing response")]
    DeseralizationError(String),

    #[error("error serializing message")]
    SeralizationError(String),

    #[error("client error")]
    ClientError(String),
}

impl From<HandlerError> for ResponseError {
    fn from(error: HandlerError) -> Self {
        ResponseError::HandlerError(error.to_string())
    }
}

/// Errors that might occur while building or using the client,
/// but that are not related no any behaviour on the server
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ClientError {
    #[error("server response error")]
    ResponseError(ResponseError),

    #[error("no servers available")]
    NoServersAvailable,

    #[error("the requested server is not available")]
    ServerNotAvailable(String),

    #[error("rendenvouz is not available")]
    RendevouzUnavailable,

    #[error("connectivity error")]
    Connectivity,

    #[error("unknown client error")]
    Unknown(String),

    #[error("unknown PlacementLock error")]
    PlacementLock,

    #[error("error deserializing response")]
    DeseralizationError(String),

    #[error("error serializing message")]
    SeralizationError(String),

    #[error("std::io::Error")]
    IoError(String),
}

impl From<::std::io::Error> for ClientError {
    fn from(error: ::std::io::Error) -> Self {
        ClientError::IoError(error.to_string())
    }
}

pub mod pubsub {
    use super::*;

    /// This is the struct that we serialize and send to the server serialized to request a new
    /// subscription
    #[derive(Debug, Serialize, Deserialize)]
    pub struct SubscriptionRequest {
        pub handler_type: String,
        pub handler_id: String,
    }

    /// Item that is streamed serialized from the server to the client
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SubscriptionResponse {
        pub body: Result<Vec<u8>, ResponseError>,
    }
}
