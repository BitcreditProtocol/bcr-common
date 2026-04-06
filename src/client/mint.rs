// ----- standard library imports
// ----- extra library imports
use thiserror::Error;
// ----- local imports
use crate::wire::swap as wire_swap;

// ----- end imports

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid request")]
    InvalidRequest,

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
    pub async fn post_swap(
        &self,
        inputs: Vec<cashu::Proof>,
        outputs: Vec<cashu::BlindedMessage>,
        commitment: bitcoin::secp256k1::schnorr::Signature,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let msg = wire_swap::SwapRequest {
            inputs,
            outputs,
            commitment,
        };
        let url = self
            .base
            .join(Self::SWAP_EP_V1)
            .expect("swap relative path");
        let request = self.cl.post(url).json(&msg);
        let response: wire_swap::SwapResponse = request.send().await?.json().await?;
        Ok(response.signatures)
    }
}
