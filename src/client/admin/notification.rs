// ----- standard library imports
// ----- extra library imports
use thiserror::Error;
// ----- local imports
use crate::{client::admin::jsonrpc, wire::notification as wire_notification};
// ----- end imports

pub mod admin_ep {
    pub const NOTIFY_V1: &str = "/admin/v1/notification";
}

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Error)]
pub enum Error {
    #[error("resource not found {0}")]
    ResourceNotFound(serde_json::Value),
    #[error("invalid request {0}")]
    InvalidRequest(serde_json::Value),
    #[error("service unavailable {0}")]
    ServiceUnavailable(serde_json::Value),
    #[error("internal {0}")]
    Internal(String),
    #[error("internal error {0}")]
    Reqwest(#[from] reqwest::Error),
}

impl std::convert::From<jsonrpc::Error> for Error {
    fn from(e: jsonrpc::Error) -> Self {
        match e {
            jsonrpc::Error::ResourceNotFound(v) => Error::ResourceNotFound(v),
            jsonrpc::Error::InvalidRequest(v) => Error::InvalidRequest(v),
            jsonrpc::Error::ServiceUnavailable(v) => Error::ServiceUnavailable(v),
            jsonrpc::Error::Internal(s) => Error::Internal(s),
            jsonrpc::Error::Reqwest(e) => Error::Reqwest(e),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    cl: jsonrpc::Client,
    base: reqwest::Url,
}

impl Client {
    pub fn new(base: reqwest::Url) -> Self {
        Self {
            cl: jsonrpc::Client::new(),
            base,
        }
    }

    pub async fn notify(&self, notification: wire_notification::NotificationRequest) -> Result<()> {
        let url = self
            .base
            .join(admin_ep::NOTIFY_V1)
            .expect("notify relative path");
        let _: wire_notification::NotificationResponse = self.cl.post(url, &notification).await?;
        Ok(())
    }
}
