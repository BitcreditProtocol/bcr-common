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
    #[error("mint operation not found {0}")]
    MintOpNotFound(uuid::Uuid),
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
    pub fn credit_unit() -> cashu::CurrencyUnit {
        cashu::CurrencyUnit::Custom(String::from(String::from("crsat")))
    }
    pub fn debit_unit() -> cashu::CurrencyUnit {
        cashu::CurrencyUnit::Sat
    }

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

    pub async fn get_or_create_credit_keyset_with_expiration(
        &self,
        expiration: chrono::NaiveDate,
    ) -> Result<cashu::KeySetInfo> {
        let unit = Self::credit_unit();
        let filters = wire_keys::KeysetInfoFilters {
            unit: Some(unit.clone()),
            min_expiration: Some(expiration - chrono::Duration::days(1)),
            max_expiration: Some(expiration + chrono::Duration::days(1)),
        };
        let kinfos = self.list_keyset_info(filters).await?;
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
        let kinfo = self.new_keyset(unit, Some(expiration), 0).await?;
        Ok(cashu::KeySetInfo::from(kinfo))
    }

    pub const KEYS_EP_V1: &'static str = "/v1/keys/{kid}";
    pub async fn keys(&self, kid: cashu::Id) -> Result<cashu::KeySet> {
        let url = self
            .base
            .join(&Self::KEYS_EP_V1.replace("{kid}", &kid.to_string()))
            .expect("keys relative path");
        let response = self.cl.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(kid));
        }
        let ks = response.json::<cashu::KeysResponse>().await?.keysets;
        ks.into_iter().next().ok_or(Error::KeysetIdNotFound(kid))
    }

    pub const LISTKEYS_EP_V1: &'static str = "/v1/keys";
    pub async fn list_keys(&self) -> Result<Vec<cashu::KeySet>> {
        let url = self
            .base
            .join(Self::LISTKEYS_EP_V1)
            .expect("list keys relative path");
        let response = self.cl.get(url).send().await?;
        let ks = response.json::<cashu::KeysResponse>().await?;
        Ok(ks.keysets)
    }

    pub const KEYSETINFO_EP_V1: &'static str = "/v1/keysets/{kid}";
    pub async fn keyset_info(&self, kid: cashu::Id) -> Result<cashu::KeySetInfo> {
        let url = self
            .base
            .join(&Self::KEYSETINFO_EP_V1.replace("{kid}", &kid.to_string()))
            .expect("keyset relative path");
        let response = self.cl.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(kid));
        }
        let ks = response.json::<cashu::KeySetInfo>().await?;
        Ok(ks)
    }

    pub const LISTKEYSETINFO_EP_V1: &'static str = "/v1/keysets";
    pub async fn list_keyset_info(
        &self,
        filters: wire_keys::KeysetInfoFilters,
    ) -> Result<Vec<cashu::KeySetInfo>> {
        let url = self
            .base
            .join(Self::LISTKEYSETINFO_EP_V1)
            .expect("keyset relative path");
        let mut request = self.cl.get(url);
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

    pub const RESTORE_EP_V1: &'static str = "/v1/restore";
    pub async fn restore(
        &self,
        outputs: Vec<cashu::BlindedMessage>,
    ) -> Result<Vec<(cashu::BlindedMessage, cashu::BlindSignature)>> {
        let url = self
            .base
            .join(Self::RESTORE_EP_V1)
            .expect("restore relative path");
        let msg = cashu::RestoreRequest { outputs };
        let response = self.cl.post(url).json(&msg).send().await?;
        let msg: cashu::RestoreResponse = response.json().await?;
        let cashu::RestoreResponse {
            outputs,
            signatures,
            ..
        } = msg;
        let ret_val = outputs
            .into_iter()
            .zip(signatures.into_iter())
            .collect::<Vec<_>>();
        Ok(ret_val)
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
