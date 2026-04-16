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
    #[error("invalid request {0}")]
    InvalidRequest(String),

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
    pub fn currency_unit() -> cashu::CurrencyUnit {
        crate::client::CURRENCY_UNIT
    }

    pub fn new(base: reqwest::Url) -> Self {
        Self {
            cl: reqwest::Client::new(),
            base,
        }
    }

    pub const NEW_KEYSET_EP_V1: &str = "/v1/admin/keys";
    pub async fn new_keyset(
        &self,
        expiration: Option<chrono::NaiveDate>,
        fees_ppk: u64,
    ) -> Result<cdk_common::mint::MintKeySetInfo> {
        let url = self
            .base
            .join(Self::NEW_KEYSET_EP_V1)
            .expect("new keys relative path");
        let request = self.cl.post(url).json(&wire_keys::NewKeysetRequest {
            unit: crate::client::CURRENCY_UNIT,
            expiration,
            fees_ppk,
        });
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest(response.text().await?));
        }
        let ks = response.json::<cdk_common::mint::MintKeySetInfo>().await?;
        Ok(ks)
    }

    pub async fn get_or_create_keyset_with_expiration(
        &self,
        expiration: chrono::NaiveDate,
    ) -> Result<cashu::KeySetInfo> {
        let unit = Self::currency_unit();
        let filters = wire_keys::KeysetInfoFilters {
            unit: Some(unit.clone()),
            min_expiration: Some(expiration - chrono::Duration::days(1)),
            max_expiration: Some(expiration + chrono::Duration::days(1)),
        };
        let kinfos = common::list_keyset_info(&self.cl, &self.base, filters).await?;
        let expiration_tstamp = u64::try_from(
            expiration
                .and_time(chrono::NaiveTime::MIN)
                .and_utc()
                .timestamp(),
        )
        .unwrap_or(0);
        for kinfo in kinfos {
            if kinfo.unit == unit && kinfo.final_expiry == Some(expiration_tstamp) {
                return Ok(kinfo);
            }
        }
        let kinfo = self.new_keyset(Some(expiration), 0).await?;
        Ok(cashu::KeySetInfo::from(kinfo))
    }

    pub const SIGN_EP_V1: &str = "/v1/admin/keys/sign";
    pub async fn sign(&self, msgs: &[cashu::BlindedMessage]) -> Result<Vec<cashu::BlindSignature>> {
        if msgs.is_empty() {
            return Ok(vec![]);
        }
        let unique_kids = msgs
            .iter()
            .map(|m| m.keyset_id)
            .collect::<std::collections::HashSet<_>>();
        if unique_kids.len() > 1 {
            return Err(Error::InvalidRequest(String::from(
                "multiple kids in blinds",
            )));
        }
        let kid = unique_kids.into_iter().next().unwrap();
        let url = self
            .base
            .join(Self::SIGN_EP_V1)
            .expect("sign relative path");
        let request = self.cl.post(url).json(msgs);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest(response.text().await?));
        }
        if response.status() == reqwest::StatusCode::CONFLICT {
            return Err(Error::InvalidRequest(response.text().await?));
        }
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(kid));
        }
        let sigs = response.json::<Vec<cashu::BlindSignature>>().await?;
        Ok(sigs)
    }

    pub const VERIFY_PROOF_EP_V1: &str = "/v1/admin/keys/verify/proof";
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
            return Err(Error::InvalidRequest(response.text().await?));
        }
        response.error_for_status()?;
        Ok(())
    }

    pub const VERIFY_FINGERPRINT_EP_V1: &str = "/v1/admin/keys/verify/fingerprint";
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
            return Err(Error::InvalidRequest(response.text().await?));
        }
        response.error_for_status()?;
        Ok(())
    }

    pub const DEACTIVATEKEYSET_EP_V1: &str = "/v1/admin/keys/deactivate";
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

    pub const BURN_EP_V1: &str = "/v1/core/burn";
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

    pub const RECOVER_EP_V1: &str = "/v1/admin/swap/recover";
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

    pub async fn keys(&self, kid: cashu::Id) -> Result<cashu::KeySet> {
        let result = common::keys(&self.cl, &self.base, kid).await?;
        Ok(result)
    }

    pub async fn list_keyset_info(
        &self,
        filters: wire_keys::KeysetInfoFilters,
    ) -> Result<Vec<cashu::KeySetInfo>> {
        let result = common::list_keyset_info(&self.cl, &self.base, filters).await?;
        Ok(result)
    }

    pub async fn keyset_info(&self, kid: cashu::Id) -> Result<cashu::KeySetInfo> {
        let result = common::keyset_info(&self.cl, &self.base, kid).await?;
        Ok(result)
    }

    pub async fn check_state(&self, ys: Vec<cashu::PublicKey>) -> Result<Vec<cashu::ProofState>> {
        let result = common::check_state(&self.cl, &self.base, ys).await?;
        Ok(result)
    }

    pub async fn swap(
        &self,
        inputs: Vec<cashu::Proof>,
        outputs: Vec<cashu::BlindedMessage>,
        commitment: bitcoin::secp256k1::schnorr::Signature,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let result = common::swap(&self.cl, &self.base, inputs, outputs, commitment).await?;
        Ok(result)
    }
}

pub(crate) mod common {
    use super::*;

    pub const LISTKEYSETINFO_EP_V1: &str = "/v1/core/keysets";
    pub async fn list_keyset_info(
        cl: &reqwest::Client,
        base: &reqwest::Url,
        filters: wire_keys::KeysetInfoFilters,
    ) -> Result<Vec<cashu::KeySetInfo>> {
        let url = base
            .join(LISTKEYSETINFO_EP_V1)
            .expect("keyset relative path");
        let mut request = cl.get(url);
        if let Some(unit) = filters.unit {
            request = request.query(&[("unit", unit.to_string())]);
        }
        if let Some(date) = filters.min_expiration {
            request = request.query(&[("min_expiration", date.to_string())]);
        }
        if let Some(date) = filters.max_expiration {
            request = request.query(&[("max_expiration", date.to_string())]);
        }
        let response = request.send().await?;
        let ks = response.json::<cashu::KeysetResponse>().await?;
        Ok(ks.keysets)
    }

    pub const KEYS_EP_V1: &str = "/v1/core/keys/{kid}";
    pub async fn keys(
        cl: &reqwest::Client,
        base: &reqwest::Url,
        kid: cashu::Id,
    ) -> Result<cashu::KeySet> {
        let url = base
            .join(&KEYS_EP_V1.replace("{kid}", &kid.to_string()))
            .expect("keys relative path");
        let response = cl.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(kid));
        }
        let ks = response.json::<cashu::KeysResponse>().await?.keysets;
        ks.into_iter().next().ok_or(Error::KeysetIdNotFound(kid))
    }

    pub const KEYSETINFO_EP_V1: &str = "/v1/core/keysets/{kid}";
    pub async fn keyset_info(
        cl: &reqwest::Client,
        base: &reqwest::Url,
        kid: cashu::Id,
    ) -> Result<cashu::KeySetInfo> {
        let url = base
            .join(&KEYSETINFO_EP_V1.replace("{kid}", &kid.to_string()))
            .expect("keyset relative path");
        let response = cl.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(kid));
        }
        let ks = response.json::<cashu::KeySetInfo>().await?;
        Ok(ks)
    }

    pub const CHECKSTATE_EP_V1: &str = "/v1/core/checkstate";
    pub async fn check_state(
        cl: &reqwest::Client,
        base: &reqwest::Url,
        ys: Vec<cashu::PublicKey>,
    ) -> Result<Vec<cashu::ProofState>> {
        let url = base
            .join(CHECKSTATE_EP_V1)
            .expect("checkstate relative path");
        let request = cashu::CheckStateRequest { ys };
        let response = cl.post(url).json(&request).send().await?;
        let state_resp: cashu::CheckStateResponse = response.json().await?;
        Ok(state_resp.states)
    }

    pub const SWAP_EP_V1: &str = "/v1/core/swap";
    pub async fn swap(
        cl: &reqwest::Client,
        base: &reqwest::Url,
        inputs: Vec<cashu::Proof>,
        outputs: Vec<cashu::BlindedMessage>,
        commitment: bitcoin::secp256k1::schnorr::Signature,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let url = base.join(SWAP_EP_V1).expect("swap relative path");
        let request = wire_swap::SwapRequest {
            inputs,
            outputs,
            commitment,
        };
        let response = cl.post(url).json(&request).send().await?;
        let signatures: wire_swap::SwapResponse = response.json().await?;
        Ok(signatures.signatures)
    }
}
