//! Repository of all error types for this crate using [thiserror]
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::protocol::ClientError;

/// Message handling error. It occurs, mostly, from the object look up to
/// the end of its execution
#[derive(Error, Debug, PartialEq, Eq)]
pub enum HandlerError {
    #[error("object not found")]
    ObjectNotFound,

    #[error("message handler not found")]
    HandlerNotFound,

    #[error("message serialization error")]
    MessageSerializationError,

    #[error("response serialization error")]
    ResponseSerializationError,

    #[error("unknown execution error")]
    Unknown,

    #[error("error caused internally by the application")]
    ApplicationError(Vec<u8>),
}

/// Represents errors that occur in the lifecyle functions of an object.
/// This is, in most of the [ServiceObject](crate::service_object::ServiceObject)
/// hook functions
#[derive(Error, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum ServiceObjectLifeCycleError {
    #[error("unknown error")]
    Unknown,
}

/// Errors triggered while building an [crate::client::Client] using
/// [crate::client::ClientBuilder]
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ClientBuilderError {
    #[error("no MembershipStorage provided")]
    NoMembershipStorage,
}

/// Represent errors that  happen during the [crate::server::Server] setup
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ServerError {
    #[error("bind")]
    Bind(String),

    #[error("client builder")]
    ClientBuilder(ClientError),

    #[error("cluster provider")]
    ClusterProviderServe(ClusterProviderServeError),

    #[error("object placement")]
    ObjectPlacement(ObjectPlacementError),

    #[error("Run")]
    Run,
}

impl From<ObjectPlacementError> for ServerError {
    fn from(err: ObjectPlacementError) -> Self {
        ServerError::ObjectPlacement(err)
    }
}

/// Error type for the cluster redevouz/membeship trait
/// ([crate::cluster::storage::MembershipStorage])
#[derive(Error, Debug, PartialEq, Eq)]
pub enum MembershipError {
    #[error("upstream error")]
    Upstream(String),

    #[error("unknown")]
    Unknown(String),

    #[error("This MembershipStorage is Read-only")]
    ReadOnly(String),
}

#[cfg(feature = "sql")]
impl From<sqlx::Error> for MembershipError {
    fn from(err: sqlx::Error) -> Self {
        MembershipError::Upstream(err.to_string())
    }
}

/// Error type for the serve function of the cluster provider algorith trait
/// ([crate::cluster::membership_protocol::ClusterProvider])
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ClusterProviderServeError {
    #[error("can't communicate with membership provider's storage")]
    MembershipProviderError(String),

    #[error("error pasing value into a SocketAddr")]
    SocketAddrParsingError,

    #[error("unknown cluster provider serve error")]
    Unknown(String),
}

impl From<MembershipError> for ClusterProviderServeError {
    fn from(err: MembershipError) -> Self {
        ClusterProviderServeError::MembershipProviderError(err.to_string())
    }
}

/// Error type for the object placement trait
/// ([crate::object_placement::ObjectPlacement])
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ObjectPlacementError {
    #[error("upstream error")]
    Upstream(String),

    #[error("unknown")]
    Unknown(String),
}

#[cfg(feature = "sql")]
impl From<sqlx::Error> for ObjectPlacementError {
    fn from(err: sqlx::Error) -> Self {
        ObjectPlacementError::Upstream(err.to_string())
    }
}

/// Error type for service object state management
#[derive(Error, Debug, PartialEq)]
pub enum LoadStateError {
    #[error("object not found")]
    ObjectNotFound,

    #[error("unknown error")]
    Unknown,

    #[error("deserialization error")]
    DeserializationError,

    #[error("serialization error")]
    SerializationError,
}
