// ----- standard library imports
// ----- extra library imports
use thiserror::Error;
use uuid::Uuid;
// ----- local imports
use crate::wire::quotes as wire_quotes;

// ----- end imports

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Error)]
pub enum Error {
    #[error("resource not found {0}")]
    ResourceNotFound(Uuid),
    #[error("invalid request")]
    InvalidRequest,
    #[error("internal {0}")]
    Reqwest(#[from] reqwest::Error),
}

#[derive(Debug, Clone)]
pub struct Client {
    cl: reqwest::Client,
    base: reqwest::Url,
}

impl Client {
    pub fn new(base: reqwest::Url) -> Self {
        Self {
            cl: reqwest::Client::new(),
            base,
        }
    }

    pub const LIST_EP_V1: &'static str = "/v1/admin/credit/quote";
    pub async fn list(
        &self,
        params: wire_quotes::ListParam,
    ) -> Result<wire_quotes::ListReplyLight> {
        let url = self
            .base
            .join(Self::LIST_EP_V1)
            .expect("list relative path");
        let mut request = self.cl.get(url);
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
            request = request.query(&[("bill_maturity_date_from", date.to_string())]);
        }
        if let Some(date) = bill_maturity_date_to {
            request = request.query(&[("bill_maturity_date_to", date.to_string())]);
        }
        if let Some(status) = status {
            request = request.query(&[("status", status.to_string())]);
        }
        if let Some(bill_id) = bill_id {
            request = request.query(&[("bill_id", bill_id)]);
        }
        if let Some(bill_drawee_id) = bill_drawee_id {
            request = request.query(&[("bill_drawee_id", bill_drawee_id)]);
        }
        if let Some(bill_drawer_id) = bill_drawer_id {
            request = request.query(&[("bill_drawer_id", bill_drawer_id)]);
        }
        if let Some(bill_payer_id) = bill_payer_id {
            request = request.query(&[("bill_payer_id", bill_payer_id)]);
        }
        if let Some(bill_holder_id) = bill_holder_id {
            request = request.query(&[("bill_holder_id", bill_holder_id)]);
        }
        if let Some(sort) = sort {
            request = request.query(&[("sort", sort.to_string())]);
        }

        let reply = request.send().await?.json().await?;
        Ok(reply)
    }

    pub const UPDATE_EP_V1: &'static str = "/v1/admin/credit/quote/{qid}";
    pub async fn deny(&self, qid: Uuid) -> Result<wire_quotes::UpdateQuoteResponse> {
        let url = self
            .base
            .join(&Self::UPDATE_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("deny quote relative path");
        let body = wire_quotes::UpdateQuoteRequest::Deny;
        let request = self.cl.patch(url).json(&body);
        let reply = request.send().await?.json().await?;
        Ok(reply)
    }

    pub async fn offer(
        &self,
        qid: Uuid,
        discounted: bitcoin::Amount,
        ttl: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<wire_quotes::UpdateQuoteResponse> {
        let url = self
            .base
            .join(&Self::UPDATE_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("offer quote relative path");
        let body = wire_quotes::UpdateQuoteRequest::Offer { discounted, ttl };
        let request = self.cl.patch(url).json(&body);
        let reply = request.send().await?.json().await?;
        Ok(reply)
    }

    pub const ADMIN_LOOKUP_EP_V1: &'static str = "/v1/admin/credit/quote/{qid}";
    pub async fn admin_lookup(&self, qid: Uuid) -> Result<wire_quotes::InfoReply> {
        let url = self
            .base
            .join(&Self::ADMIN_LOOKUP_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("admin lookup relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(qid));
        }
        let reply = response.json::<wire_quotes::InfoReply>().await?;
        Ok(reply)
    }
}
