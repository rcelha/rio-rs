use thiserror::Error;

use crate::protocol::ClientError;

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

    #[error("lifecycle error")]
    LyfecycleError(ServiceObjectLifeCycleError),

    #[error("unknown execution error")]
    Unknown,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ServiceObjectLifeCycleError {
    #[error("unknown error")]
    Unknown,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ClientBuilderError {
    #[error("no MembersStorage provided")]
    NoMembersStorage,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ServerBuilderError {
    #[error("no MembersStorage provided")]
    NoMembersStorage,

    #[error("no ObjectPlacementProvider")]
    NoObjectPlacementProvider,

    #[error("unknown")]
    Unknown(String),
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ServerError {
    #[error("bind")]
    Bind(String),

    #[error("client builder")]
    ClientBuilder(ClientError),

    #[error("cluster provider")]
    ClusterProviderServe(ClusterProviderServeError),

    #[error("Run")]
    Run,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum MembershipError {
    #[error("upstream error")]
    Upstream(String),

    #[error("unknown")]
    Unknown(String),
}

impl From<sqlx::Error> for MembershipError {
    fn from(err: sqlx::Error) -> Self {
        MembershipError::Upstream(err.to_string())
    }
}

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

#[derive(Error, Debug)]
pub enum LoadStateError {
    #[error("object not found")]
    ObjectNotFound,

    #[error("unknown error")]
    Unknown,
}
