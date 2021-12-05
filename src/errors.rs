use thiserror::Error;

#[derive(Error, Debug)]
pub enum HandlerError {
    #[error("object not found")]
    ObjectNotFound,
    #[error("message handler not found")]
    HandlerNotFound,

    #[error("response serialization error")]
    ResponseSerializationError,

    #[error("unknown execution error")]
    Unknown,
}
