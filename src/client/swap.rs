// ----- standard library imports
// ----- extra library imports
use thiserror::Error;
// ----- local imports
use crate::wire::swap as wire_swap;

// ----- end imports

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Error)]
pub enum Error {
    #[error("internal error {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("authorization {0}")]
    Auth(String),
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

    pub const SWAP_EP_V1: &'static str = "/v1/swap";
    pub async fn swap(
        &self,
        inputs: Vec<cashu::Proof>,
        outputs: Vec<cashu::BlindedMessage>,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let url = self
            .base
            .join(Self::SWAP_EP_V1)
            .expect("swap relative path");
        let request = cashu::SwapRequest::new(inputs, outputs);
        let res = self.cl.post(url).json(&request).send().await?;
        let signatures: cashu::SwapResponse = res.json().await?;
        Ok(signatures.signatures)
    }

    pub const BURN_EP_V1: &'static str = "/v1/burn";
    pub async fn burn(&self, proofs: Vec<cashu::Proof>) -> Result<Vec<cashu::PublicKey>> {
        let url = self
            .base
            .join(Self::BURN_EP_V1)
            .expect("burn relative path");
        let request = wire_swap::BurnRequest { proofs };
        let res = self.cl.post(url).json(&request).send().await?;
        let burn_resp: wire_swap::BurnResponse = res.json().await?;
        Ok(burn_resp.ys)
    }

    pub const RECOVER_EP_V1: &'static str = "/v1/admin/swap/recover";
    #[cfg(feature = "authorized")]
    pub async fn recover(&self, proofs: Vec<cashu::Proof>) -> Result<wire_swap::RecoverResponse> {
        let url = self
            .base
            .join(Self::RECOVER_EP_V1)
            .expect("recover relative path");
        let msg = wire_swap::RecoverRequest { proofs };
        let request = self.cl.post(url).json(&msg);
        let response = self.auth.authorize(request).send().await?.json().await?;
        Ok(response)
    }

    pub const CHECKSTATE_EP_V1: &'static str = "/v1/checkstate";
    pub async fn check_state(&self, ys: Vec<cashu::PublicKey>) -> Result<Vec<cashu::ProofState>> {
        let url = self
            .base
            .join(Self::CHECKSTATE_EP_V1)
            .expect("checkstate relative path");
        let request = cashu::CheckStateRequest { ys };
        let res = self.cl.post(url).json(&request).send().await?;
        let state_resp: cashu::CheckStateResponse = res.json().await?;
        Ok(state_resp.states)
    }
}
