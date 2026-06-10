// ----- standard library imports
// ----- extra library imports
use async_nats::jetstream::context::PublishError;
use async_nats::{ConnectError, RequestError};
use thiserror::Error;
// ----- project imports
// ----- end imports

#[derive(Debug, Error)]
pub enum ClowderClientError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("NATS Publish error: {0}")]
    NatsPublish(#[from] PublishError),

    #[error("NATS Connect error: {0}")]
    NatsConnect(#[from] ConnectError),

    #[error("NATS request error: {0}")]
    NatsRequest(#[from] RequestError),

    #[error("CBOR serialization error: {0}")]
    CborSerialization(#[from] ciborium::ser::Error<std::io::Error>),

    #[error("CBOR deserialization error: {0}")]
    CborDeserialization(#[from] ciborium::de::Error<std::io::Error>),

    #[error("IO error : {0}")]
    IOErr(#[from] std::io::Error),

    #[error("URL parsing failed: {0}")]
    UrlParse(String),

    #[error("Invalid Signature")]
    InvalidSignature,

    #[error("Invalid Public Key")]
    InvalidPublicKey,

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),
}

pub type Result<T> = std::result::Result<T, ClowderClientError>;
