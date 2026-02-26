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
        let response = self.cl.post(url).json(&request).send().await?;
        let signatures: cashu::SwapResponse = response.json().await?;
        Ok(signatures.signatures)
    }

    pub const BURN_EP_V1: &'static str = "/v1/burn";
    pub async fn burn(&self, proofs: Vec<cashu::Proof>) -> Result<Vec<cashu::PublicKey>> {
        let url = self
            .base
            .join(Self::BURN_EP_V1)
            .expect("burn relative path");
        let request = wire_swap::BurnRequest { proofs };
        let response = self.cl.post(url).json(&request).send().await?;
        let burn_resp: wire_swap::BurnResponse = response.json().await?;
        Ok(burn_resp.ys)
    }

    pub const RECOVER_EP_V1: &'static str = "/v1/admin/swap/recover";
    pub async fn recover(&self, proofs: Vec<cashu::Proof>) -> Result<wire_swap::RecoverResponse> {
        let url = self
            .base
            .join(Self::RECOVER_EP_V1)
            .expect("recover relative path");
        let msg = wire_swap::RecoverRequest { proofs };
        let request = self.cl.post(url).json(&msg);
        let response = request.send().await?.json().await?;
        Ok(response)
    }

    pub const CHECKSTATE_EP_V1: &'static str = "/v1/checkstate";
    pub async fn check_state(&self, ys: Vec<cashu::PublicKey>) -> Result<Vec<cashu::ProofState>> {
        let url = self
            .base
            .join(Self::CHECKSTATE_EP_V1)
            .expect("checkstate relative path");
        let request = cashu::CheckStateRequest { ys };
        let response = self.cl.post(url).json(&request).send().await?;
        let state_resp: cashu::CheckStateResponse = response.json().await?;
        Ok(state_resp.states)
    }
}
