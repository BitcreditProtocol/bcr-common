// ----- standard library imports
// ----- extra library imports
use thiserror::Error;
// ----- local imports
use crate::wire::{keys as wire_keys, swap as wire_swap};

// ----- end imports

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("resource not found {0}")]
    KeysetIdNotFound(cashu::Id),
    #[error("invalid request")]
    InvalidRequest,

    #[error("internal error {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("sign error {0}")]
    NUT20(#[from] cashu::nut20::Error),
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

    pub const NEW_KEYSET_EP_V1: &'static str = "/v1/admin/keys";
    pub async fn new_keyset(
        &self,
        unit: cashu::CurrencyUnit,
        expiration: Option<chrono::NaiveDate>,
        fees_ppk: u64,
    ) -> Result<cdk_common::mint::MintKeySetInfo> {
        let url = self
            .base
            .join(Self::NEW_KEYSET_EP_V1)
            .expect("new keys relative path");
        let request = self.cl.post(url).json(&wire_keys::NewKeysetRequest {
            unit,
            expiration,
            fees_ppk,
        });
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        let ks = response.json::<cdk_common::mint::MintKeySetInfo>().await?;
        Ok(ks)
    }

    pub const SIGN_EP_V1: &'static str = "/v1/admin/keys/sign";
    pub async fn sign(&self, msgs: &[cashu::BlindedMessage]) -> Result<Vec<cashu::BlindSignature>> {
        if msgs.is_empty() {
            return Ok(vec![]);
        }
        let unique_kids = msgs
            .iter()
            .map(|m| m.keyset_id)
            .collect::<std::collections::HashSet<_>>();
        if unique_kids.len() > 1 {
            return Err(Error::InvalidRequest);
        }
        let kid = unique_kids.into_iter().next().unwrap();
        let url = self
            .base
            .join(Self::SIGN_EP_V1)
            .expect("sign relative path");
        let request = self.cl.post(url).json(msgs);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        if response.status() == reqwest::StatusCode::CONFLICT {
            return Err(Error::InvalidRequest);
        }
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(kid));
        }
        let sigs = response.json::<Vec<cashu::BlindSignature>>().await?;
        Ok(sigs)
    }

    pub const VERIFY_PROOF_EP_V1: &'static str = "/v1/admin/keys/verify/proof";
    pub async fn verify_proof(&self, proof: &cashu::Proof) -> Result<()> {
        let url = self
            .base
            .join(Self::VERIFY_PROOF_EP_V1)
            .expect("verify relative path");
        let request = self.cl.post(url).json(proof);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(proof.keyset_id));
        }
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        response.error_for_status()?;
        Ok(())
    }

    pub const VERIFY_FINGERPRINT_EP_V1: &'static str = "/v1/admin/keys/verify/fingerprint";
    pub async fn verify_fingerprint(&self, fp: &wire_keys::ProofFingerprint) -> Result<()> {
        let url = self
            .base
            .join(Self::VERIFY_FINGERPRINT_EP_V1)
            .expect("verify relative path");
        let request = self.cl.post(url).json(fp);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(fp.keyset_id));
        }
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        response.error_for_status()?;
        Ok(())
    }

    pub const DEACTIVATEKEYSET_EP_V1: &'static str = "/v1/admin/keys/deactivate";
    pub async fn deactivate_keyset(&self, kid: cashu::Id) -> Result<cashu::Id> {
        let url = self
            .base
            .join(Self::DEACTIVATEKEYSET_EP_V1)
            .expect("deactivate relative path");
        let msg = wire_keys::DeactivateKeysetRequest { kid };
        let request = self.cl.post(url).json(&msg);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(kid));
        }
        let response: wire_keys::DeactivateKeysetResponse = response.json().await?;
        Ok(response.kid)
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
}
