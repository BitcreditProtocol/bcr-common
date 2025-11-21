// ----- standard library imports
// ----- extra library imports
use thiserror::Error;
use uuid::Uuid;
// ----- local imports
use crate::{core::signature::serialize_n_schnorr_sign_borsh_msg, wire::quotes as wire_quotes};

// ----- end imports

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Error)]
pub enum Error {
    #[error("resource not found {0}")]
    ResourceNotFound(Uuid),
    #[error("invalid request")]
    InvalidRequest,
    #[error("signature {0}")]
    Signature(#[from] crate::core::signature::BorshMsgSignatureError),
    #[error("authorization {0}")]
    Auth(String),
    #[error("internal {0}")]
    Reqwest(#[from] reqwest::Error),
}

#[cfg(feature = "authorized")]
impl std::convert::From<crate::client::authorization::Error> for Error {
    fn from(e: crate::client::authorization::Error) -> Self {
        match e {
            crate::client::authorization::Error::Reqwest(e) => Error::Reqwest(e),
            _ => Error::Auth(e.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    cl: reqwest::Client,
    base: reqwest::Url,
    #[cfg(feature = "authorized")]
    auth: std::sync::Arc<crate::client::authorization::AuthorizationPlugin>,
}

impl Client {
    pub fn new(base: reqwest::Url) -> Self {
        Self {
            cl: reqwest::Client::new(),
            base,
            #[cfg(feature = "authorized")]
            auth: Default::default(),
        }
    }

    #[cfg(feature = "authorized")]
    pub async fn authenticate(
        &mut self,
        token_url: reqwest::Url,
        client_id: &str,
        client_secret: &str,
        username: &str,
        password: &str,
    ) -> Result<()> {
        self.auth
            .authenticate(
                &self.cl,
                token_url,
                client_id,
                client_secret,
                username,
                password,
            )
            .await?;
        Ok(())
    }

    #[cfg(feature = "authorized")]
    pub async fn refresh_access_token(&self, client_id: String) -> Result<std::time::Duration> {
        let exp = self.auth.refresh_access_token(&self.cl, client_id).await?;
        Ok(exp)
    }

    pub const ENQUIRE_EP_V1: &'static str = "/v1/mint/quote/credit";
    pub async fn enquire(
        &self,
        bill: wire_quotes::SharedBill,
        minting_pubkey: cashu::PublicKey,
        signing_key: &bitcoin::secp256k1::Keypair,
    ) -> Result<Uuid> {
        let request = wire_quotes::EnquireRequest {
            content: bill,
            minting_pubkey,
        };
        let (content, signature) = serialize_n_schnorr_sign_borsh_msg(&request, signing_key)?;
        let signed = wire_quotes::SignedEnquireRequest { content, signature };
        let url = self
            .base
            .join(Self::ENQUIRE_EP_V1)
            .expect("enquire relative path");
        let res = self.cl.post(url).json(&signed).send().await?;
        let reply = res.json::<wire_quotes::EnquireReply>().await?;
        Ok(reply.id)
    }

    pub const LOOKUP_EP_V1: &'static str = "/v1/mint/quote/credit/{qid}";
    pub async fn lookup(&self, qid: Uuid) -> Result<wire_quotes::StatusReply> {
        let url = self
            .base
            .join(&Self::LOOKUP_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("lookup relative path");
        let res = self.cl.get(url).send().await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(qid));
        }
        let reply = res.json::<wire_quotes::StatusReply>().await?;
        Ok(reply)
    }

    pub const LIST_EP_V1: &'static str = "/v1/admin/credit/quote";
    #[cfg(feature = "authorized")]
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
            ..
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

        let reply = self.auth.authorize(request).send().await?.json().await?;
        Ok(reply)
    }

    pub const UPDATE_EP_V1: &'static str = "/v1/admin/credit/quote/{qid}";
    #[cfg(feature = "authorized")]
    pub async fn deny(&self, qid: Uuid) -> Result<wire_quotes::UpdateQuoteResponse> {
        let url = self
            .base
            .join(&Self::UPDATE_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("deny quote relative path");
        let body = wire_quotes::UpdateQuoteRequest::Deny;
        let request = self.cl.put(url).json(&body);
        let reply = self.auth.authorize(request).send().await?.json().await?;
        Ok(reply)
    }

    #[cfg(feature = "authorized")]
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
        let request = self.cl.put(url).json(&body);
        let reply = self.auth.authorize(request).send().await?.json().await?;
        Ok(reply)
    }

    pub const RESOLVE_EP_V1: &'static str = "/v1/mint/quote/credit/{qid}";
    pub async fn accept_offer(&self, qid: Uuid) -> Result<()> {
        let url = self
            .base
            .join(&Self::RESOLVE_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("accept offer relative path");
        let res = self
            .cl
            .post(url)
            .json(&wire_quotes::ResolveOffer::Accept)
            .send()
            .await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(qid));
        }
        Ok(())
    }

    pub async fn reject_offer(&self, qid: Uuid) -> Result<()> {
        let url = self
            .base
            .join(&Self::RESOLVE_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("reject offer relative path");
        let res = self
            .cl
            .post(url)
            .json(&wire_quotes::ResolveOffer::Reject)
            .send()
            .await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(qid));
        }
        Ok(())
    }

    pub async fn cancel_enquiry(&self, qid: Uuid) -> Result<()> {
        let url = self
            .base
            .join(&Self::RESOLVE_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("cancel enquiry relative path");
        let res = self.cl.delete(url).send().await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(qid));
        }
        Ok(())
    }

    pub const ENABLE_MINTING_EP_V1: &'static str = "/v1/admin/credit/quote/enable_mint/{qid}";
    #[cfg(feature = "authorized")]
    pub async fn enable_minting(&self, qid: Uuid) -> Result<wire_quotes::EnableMintingResponse> {
        let url = self
            .base
            .join(&Self::ENABLE_MINTING_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("enable minting relative path");
        let body = wire_quotes::EnableMintingRequest {};
        let request = self.cl.post(url).json(&body);
        let reply = self.auth.authorize(request).send().await?.json().await?;
        Ok(reply)
    }
}
