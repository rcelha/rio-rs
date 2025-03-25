//! Client/Server communication protocol

use super::errors::HandlerError;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
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
///
/// ```rust
/// # use rio_rs::protocol::*;
///
/// // Success case
/// let response = ResponseEnvelope::new(vec![1, 2, 3]);
/// assert!(response.body.is_ok());
///
/// // Error case
/// let response_error = ResponseError::Unknown("something went wrong".to_string());
/// let response = ResponseEnvelope::err(response_error);
/// assert!(response.body.is_err());
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub body: Result<Vec<u8>, ResponseError>,
}

impl ResponseEnvelope {
    /// New `ResponseEnvelope`. `Ok` Variant
    pub fn new(body: Vec<u8>) -> ResponseEnvelope {
        ResponseEnvelope { body: Ok(body) }
    }

    /// New `ResponseEnvelope`. `Err` Variant
    pub fn err(error: ResponseError) -> ResponseEnvelope {
        ResponseEnvelope { body: Err(error) }
    }
}

/// Convert a `HandlerError` into a `ResponseEnvelope`.
///
/// This is used to convert errors that occur during handler execution
/// into a response envelope that can be sent back to the client.
impl From<HandlerError> for ResponseEnvelope {
    fn from(error: HandlerError) -> Self {
        let response_err = ResponseError::from(error);
        ResponseEnvelope {
            body: Err(response_err),
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

    #[error("ServiceObject could not be allocated")]
    Allocate,

    #[error("ServiceObject not supported")]
    NotSupported(String),

    #[error("unknown execution error")]
    Unknown(String),

    #[error("handler error")]
    HandlerError(String),

    #[error("error deserializing response")]
    DeseralizationError(String),

    #[error("error serializing message")]
    SeralizationError(String),

    #[error("Error caused by the application, serialized in bincode")]
    ApplicationError(Vec<u8>),
}

/// Convert a `HandlerError` into a `ResponseError`.
///
/// This is used to convert errors that occur during handler execution
/// into a response error that can be serialized and sent back to the client.
impl From<HandlerError> for ResponseError {
    fn from(error: HandlerError) -> Self {
        match error {
            HandlerError::ApplicationError(v) => ResponseError::ApplicationError(v),
            inner_err => ResponseError::Unknown(inner_err.to_string()),
        }
    }
}

/// Errors that might occur while building or using the client,
/// but that are not related to any behaviour on the server
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ClientError {
    #[error("no servers available")]
    NoServersAvailable,

    #[error("the requested server is not available")]
    ServerNotAvailable(String),

    #[error("client was disconnected from the server (no more items on the TCP stream)")]
    Disconnect,

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

/// This error can be used with [RequestError] when you don't care
/// about the possible application errors
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum NoopError {}

/// Union of types for actions that bundle client logic and response handling
#[derive(Error, Debug, PartialEq, Eq)]
pub enum RequestError<E: std::error::Error> {
    #[error("error in the service response")]
    ResponseError(ResponseError),

    #[error("client error")]
    ClientError(ClientError),

    #[error("application error")]
    ApplicationError(E),
}

impl<E: std::error::Error> From<::std::io::Error> for RequestError<E> {
    fn from(error: ::std::io::Error) -> Self {
        Into::<ClientError>::into(error).into()
    }
}

/// Convert a `ClientError` into a `RequestError`.
///
/// This is useful because client-side operations can fail for reasons that are not
/// related to the server's response (e.g., network connectivity issues,
/// deserialization errors on the client-side, etc.).
///
/// By implementing `From<ClientError> for RequestError`, we can easily propagate
/// client-side errors up to the request handling layer, where they can be
/// treated as part of the overall request error scenario, without needing to
/// explicitly convert them everywhere they might occur.
impl<E: std::error::Error> From<ClientError> for RequestError<E> {
    fn from(err: ClientError) -> Self {
        RequestError::ClientError(err)
    }
}

impl<E: std::error::Error + DeserializeOwned> From<ResponseError> for RequestError<E> {
    fn from(err: ResponseError) -> Self {
        match err {
            ResponseError::ApplicationError(ser_error) => {
                //
                let des_result: Result<E, _> = bincode::deserialize(&ser_error);
                match des_result {
                    Ok(err) => Self::ApplicationError(err),
                    Err(bincode_err) => {
                        let error_message =
                            format!("Application error deserialization issue: {}", bincode_err);
                        let des_error = ResponseError::DeseralizationError(error_message);
                        Self::ResponseError(des_error)
                    }
                }
            }
            rest => RequestError::ResponseError(rest),
        }
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

    impl SubscriptionResponse {
        /// New `Ok` variant
        pub fn new(body: Vec<u8>) -> Self {
            SubscriptionResponse { body: Ok(body) }
        }

        /// New `Err` Variant
        pub fn err(error: ResponseError) -> Self {
            SubscriptionResponse { body: Err(error) }
        }
    }
}
