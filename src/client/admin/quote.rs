// ----- standard library imports
// ----- extra library imports
use thiserror::Error;
use uuid::Uuid;
// ----- local imports
use crate::{client::admin::jsonrpc, wire::quotes as wire_quotes};

// ----- end imports

pub mod admin_ep {
    pub const LIST_V1: &str = "/v1/admin/quote";
    pub const LOOKUP_V1: &str = "/v1/admin/quote/{qid}";
    pub const UPDATE_V1: &str = "/v1/admin/quote/{qid}";
}

pub mod web_ep {
    pub const ENQUIRE_V1: &str = "/v1/ebill";
    pub const ENQUIRE_V1_EXT: &str = "/v1/quote/ebill";
    pub const LOOKUP_V1: &str = "/v1/ebill/{qid}";
    pub const LOOKUP_V1_EXT: &str = "/v1/quote/ebill/{qid}";
    pub const RESOLVE_V1: &str = "/v1/ebill/{qid}";
    pub const RESOLVE_V1_EXT: &str = "/v1/quote/ebill/{qid}";
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("resource not found {0}")]
    ResourceNotFound(String),
    #[error("invalid request {0}")]
    InvalidRequest(String),
    #[error("internal {0}")]
    Internal(String),
    #[error("internal error {0}")]
    Reqwest(#[from] reqwest::Error),
}

impl std::convert::From<jsonrpc::Error> for Error {
    fn from(value: jsonrpc::Error) -> Self {
        match value {
            jsonrpc::Error::ResourceNotFound(msg) => Self::ResourceNotFound(msg),
            jsonrpc::Error::InvalidRequest(msg) => Self::InvalidRequest(msg),
            jsonrpc::Error::Internal(msg) => Self::Internal(msg),
            jsonrpc::Error::Reqwest(err) => Self::Reqwest(err),
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

    pub async fn list(
        &self,
        params: wire_quotes::ListParam,
    ) -> Result<wire_quotes::ListReplyLight> {
        let url = self
            .base
            .join(admin_ep::LIST_V1)
            .expect("list relative path");
        let mut queries: Vec<(&'static str, String)> = vec![];
        let wire_quotes::ListParam {
            bill_maturity_date_from,
            bill_maturity_date_to,
            status,
            bill_id,
            bill_drawee_id,
            bill_drawer_id,
            bill_payer_id,
            bill_holder_id,
            sort,
        } = params;
        if let Some(date) = bill_maturity_date_from {
            queries.push(("bill_maturity_date_from", date.to_string()));
        }
        if let Some(date) = bill_maturity_date_to {
            queries.push(("bill_maturity_date_to", date.to_string()));
        }
        if let Some(status) = status {
            queries.push(("status", status.to_string()));
        }
        if let Some(bill_id) = bill_id {
            queries.push(("bill_id", bill_id.to_string()));
        }
        if let Some(bill_drawee_id) = bill_drawee_id {
            queries.push(("bill_drawee_id", bill_drawee_id.to_string()));
        }
        if let Some(bill_drawer_id) = bill_drawer_id {
            queries.push(("bill_drawer_id", bill_drawer_id.to_string()));
        }
        if let Some(bill_payer_id) = bill_payer_id {
            queries.push(("bill_payer_id", bill_payer_id.to_string()));
        }
        if let Some(bill_holder_id) = bill_holder_id {
            queries.push(("bill_holder_id", bill_holder_id.to_string()));
        }
        if let Some(sort) = sort {
            queries.push(("sort", sort.to_string()));
        }
        let reply = self.cl.get(url, &queries).await?;
        Ok(reply)
    }

    pub async fn deny(&self, qid: Uuid) -> Result<wire_quotes::UpdateQuoteResponse> {
        assert!(admin_ep::UPDATE_V1.contains("{qid}"));
        let url = self
            .base
            .join(&admin_ep::UPDATE_V1.replace("{qid}", &qid.to_string()))
            .expect("deny quote relative path");
        let body = wire_quotes::UpdateQuoteRequest::Deny;
        let response = self.cl.patch(url, &body).await?;
        Ok(response)
    }

    pub async fn offer(
        &self,
        qid: Uuid,
        discounted: bitcoin::Amount,
        ttl: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<wire_quotes::UpdateQuoteResponse> {
        assert!(admin_ep::UPDATE_V1.contains("{qid}"));
        let url = self
            .base
            .join(&admin_ep::UPDATE_V1.replace("{qid}", &qid.to_string()))
            .expect("offer quote relative path");
        let body = wire_quotes::UpdateQuoteRequest::Offer { discounted, ttl };
        let response = self.cl.patch(url, &body).await?;
        Ok(response)
    }

    pub async fn lookup(&self, qid: Uuid) -> Result<wire_quotes::InfoReply> {
        assert!(admin_ep::LOOKUP_V1.contains("{qid}"));
        let url = self
            .base
            .join(&admin_ep::LOOKUP_V1.replace("{qid}", &qid.to_string()))
            .expect("admin lookup relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }
}
