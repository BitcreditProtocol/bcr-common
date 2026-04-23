// ----- standard library imports
// ----- extra library imports
use thiserror::Error;
// ----- local imports
use crate::{
    client::admin::jsonrpc,
    wire::{keys as wire_keys, swap as wire_swap},
};

// ----- end imports

pub mod admin_ep {
    pub const NEW_KEYSET_V1: &str = "/v1/admin/keys";
    pub const SIGN_V1: &str = "/v1/admin/keys/sign";
    pub const VERIFY_PROOF_V1: &str = "/v1/admin/keys/verify/proof";
    pub const VERIFY_FINGERPRINT_V1: &str = "/v1/admin/keys/verify/fingerprint";
    pub const DEACTIVATE_KEYSET_V1: &str = "/v1/admin/keys/deactivate";
    pub const RECOVER_V1: &str = "/v1/admin/swap/recover";
    pub const BURN_V1: &str = "/v1/admin/burn";
}

pub mod web_ep {
    pub const LIST_KEYSET_INFO_V1: &str = "/v1/keysets";
    pub const LIST_KEYSET_INFO_V1_EXT: &str = "/v1/core/keysets";
    pub const KEYS_V1: &str = "/v1/keys/{kid}";
    pub const KEYS_V1_EXT: &str = "/v1/core/keys/{kid}";
    pub const KEYSET_INFO_V1: &str = "/v1/keysets/{kid}";
    pub const KEYSET_INFO_V1_EXT: &str = "/v1/core/keysets/{kid}";
    pub const CHECK_STATE_V1: &str = "/v1/checkstate";
    pub const CHECK_STATE_V1_EXT: &str = "/v1/core/checkstate";
    pub const SWAP_V1: &str = "/v1/swap";
    pub const SWAP_V1_EXT: &str = "/v1/core/swap";
    pub const SWAP_COMMIT_V1: &str = "/v1/swap/commit";
    pub const SWAP_COMMIT_V1_EXT: &str = "/v1/core/swap/commit";
    pub const RESTORE_V1: &str = "/v1/restore";
    pub const RESTORE_V1_EXT: &str = "/v1/core/restore";
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

    #[error("sign error {0}")]
    NUT20(#[from] cashu::nut20::Error),
    #[error("borsh sign error {0}")]
    BorshSign(#[from] crate::core::signature::BorshMsgSignatureError),
}

impl std::convert::From<jsonrpc::Error> for Error {
    fn from(value: jsonrpc::Error) -> Self {
        match value {
            jsonrpc::Error::ResourceNotFound(msg) => Self::ResourceNotFound(msg),
            jsonrpc::Error::InvalidRequest(msg) => Self::InvalidRequest(msg),
            jsonrpc::Error::Internal(msg) => Self::InvalidRequest(msg),
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
    pub fn currency_unit() -> cashu::CurrencyUnit {
        crate::client::CURRENCY_UNIT
    }

    pub fn new(base: reqwest::Url) -> Self {
        Self {
            cl: jsonrpc::Client::new(),
            base,
        }
    }

    pub async fn new_keyset(
        &self,
        expiration: Option<chrono::NaiveDate>,
        fees_ppk: u64,
    ) -> Result<cdk_common::mint::MintKeySetInfo> {
        let url = self
            .base
            .join(admin_ep::NEW_KEYSET_V1)
            .expect("new keys relative path");
        let request = wire_keys::NewKeysetRequest {
            unit: crate::client::CURRENCY_UNIT,
            expiration,
            fees_ppk,
        };
        let ks = self.cl.post(url, &request).await?;
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
        let kinfos =
            common::list_keyset_info(&self.cl, &self.base, web_ep::LIST_KEYSET_INFO_V1, filters)
                .await?;
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
        let url = self
            .base
            .join(admin_ep::SIGN_V1)
            .expect("sign relative path");
        let sigs = self.cl.post(url, msgs).await?;
        Ok(sigs)
    }

    pub async fn verify_proof(&self, proof: &cashu::Proof) -> Result<()> {
        let url = self
            .base
            .join(admin_ep::VERIFY_PROOF_V1)
            .expect("verify relative path");
        self.cl.post_no_response(url, proof).await?;
        Ok(())
    }

    pub async fn verify_fingerprint(&self, fp: &wire_keys::ProofFingerprint) -> Result<()> {
        let url = self
            .base
            .join(admin_ep::VERIFY_FINGERPRINT_V1)
            .expect("verify relative path");
        self.cl.post_no_response(url, fp).await?;
        Ok(())
    }

    pub async fn deactivate_keyset(&self, kid: cashu::Id) -> Result<cashu::Id> {
        let url = self
            .base
            .join(admin_ep::DEACTIVATE_KEYSET_V1)
            .expect("deactivate relative path");
        let msg = wire_keys::DeactivateKeysetRequest { kid };
        let response: wire_keys::DeactivateKeysetResponse = self.cl.post(url, &msg).await?;
        Ok(response.kid)
    }

    pub async fn burn(&self, proofs: Vec<cashu::Proof>) -> Result<Vec<cashu::PublicKey>> {
        let url = self
            .base
            .join(admin_ep::BURN_V1)
            .expect("burn relative path");
        let request = wire_swap::BurnRequest { proofs };
        let burn_resp: wire_swap::BurnResponse = self.cl.post(url, &request).await?;
        Ok(burn_resp.ys)
    }

    pub async fn recover(&self, proofs: Vec<cashu::Proof>) -> Result<wire_swap::RecoverResponse> {
        let url = self
            .base
            .join(admin_ep::RECOVER_V1)
            .expect("recover relative path");
        let msg = wire_swap::RecoverRequest { proofs };
        let response = self.cl.post(url, &msg).await?;
        Ok(response)
    }

    pub async fn keys(&self, kid: cashu::Id) -> Result<cashu::KeySet> {
        let result = common::keys(&self.cl, &self.base, web_ep::KEYS_V1, kid).await?;
        Ok(result)
    }

    pub async fn list_keyset_info(
        &self,
        filters: wire_keys::KeysetInfoFilters,
    ) -> Result<Vec<cashu::KeySetInfo>> {
        let result =
            common::list_keyset_info(&self.cl, &self.base, web_ep::LIST_KEYSET_INFO_V1, filters)
                .await?;
        Ok(result)
    }

    pub async fn keyset_info(&self, kid: cashu::Id) -> Result<cashu::KeySetInfo> {
        let result = common::keyset_info(&self.cl, &self.base, web_ep::KEYSET_INFO_V1, kid).await?;
        Ok(result)
    }

    pub async fn check_state(&self, ys: Vec<cashu::PublicKey>) -> Result<Vec<cashu::ProofState>> {
        let result = common::check_state(&self.cl, &self.base, web_ep::CHECK_STATE_V1, ys).await?;
        Ok(result)
    }

    pub async fn commit_swap(
        &self,
        inputs: Vec<wire_keys::ProofFingerprint>,
        outputs: Vec<cashu::BlindedMessage>,
        expiry: u64,
        wallet_pk: bitcoin::secp256k1::PublicKey,
        mint_pk: bitcoin::secp256k1::PublicKey,
    ) -> Result<bitcoin::secp256k1::schnorr::Signature> {
        let result = common::commit_swap(
            &self.cl,
            &self.base,
            web_ep::SWAP_COMMIT_V1,
            inputs,
            outputs,
            expiry,
            wallet_pk,
            mint_pk,
        )
        .await?;
        Ok(result)
    }

    pub async fn swap(
        &self,
        inputs: Vec<cashu::Proof>,
        outputs: Vec<cashu::BlindedMessage>,
        commitment: bitcoin::secp256k1::schnorr::Signature,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let result = common::swap(
            &self.cl,
            &self.base,
            web_ep::SWAP_V1,
            inputs,
            outputs,
            commitment,
        )
        .await?;
        Ok(result)
    }
}

pub(crate) mod common {
    use super::*;

    #[inline]
    pub async fn list_keyset_info(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        filters: wire_keys::KeysetInfoFilters,
    ) -> Result<Vec<cashu::KeySetInfo>> {
        let url = base.join(ep).expect("keyset relative path");
        let mut queries: Vec<(&'static str, String)> = vec![];
        if let Some(unit) = filters.unit {
            queries.push(("unit", unit.to_string()));
        }
        if let Some(date) = filters.min_expiration {
            queries.push(("min_expiration", date.to_string()));
        }
        if let Some(date) = filters.max_expiration {
            queries.push(("max_expiration", date.to_string()));
        }
        let response: cashu::KeysetResponse = cl.get(url, &queries).await?;
        Ok(response.keysets)
    }

    #[inline]
    pub async fn keys(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        kid: cashu::Id,
    ) -> Result<cashu::KeySet> {
        assert!(ep.contains("{kid}"));
        let url = base
            .join(&ep.replace("{kid}", &kid.to_string()))
            .expect("keys relative path");
        let response: cashu::KeysResponse = cl.get(url, &[]).await?;
        response
            .keysets
            .into_iter()
            .next()
            .ok_or(Error::ResourceNotFound(kid.to_string()))
    }

    #[inline]
    pub async fn keyset_info(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        kid: cashu::Id,
    ) -> Result<cashu::KeySetInfo> {
        assert!(ep.contains("{kid}"));
        let url = base
            .join(&ep.replace("{kid}", &kid.to_string()))
            .expect("keyset relative path");
        let response: cashu::KeySetInfo = cl.get(url, &[]).await?;
        Ok(response)
    }

    #[inline]
    pub async fn check_state(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        ys: Vec<cashu::PublicKey>,
    ) -> Result<Vec<cashu::ProofState>> {
        let url = base.join(ep).expect("checkstate relative path");
        let request = cashu::CheckStateRequest { ys };
        let response: cashu::CheckStateResponse = cl.post(url, &request).await?;
        Ok(response.states)
    }

    #[inline]
    pub async fn swap(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        inputs: Vec<cashu::Proof>,
        outputs: Vec<cashu::BlindedMessage>,
        commitment: bitcoin::secp256k1::schnorr::Signature,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let url = base.join(ep).expect("swap relative path");
        let request = wire_swap::SwapRequest {
            inputs,
            outputs,
            commitment,
        };
        let response: wire_swap::SwapResponse = cl.post(url, &request).await?;
        Ok(response.signatures)
    }

    #[inline]
    pub async fn commit_swap(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        inputs: Vec<wire_keys::ProofFingerprint>,
        outputs: Vec<cashu::BlindedMessage>,
        expiry: u64,
        wallet_pk: bitcoin::secp256k1::PublicKey,
        mint_pk: bitcoin::secp256k1::PublicKey,
    ) -> Result<bitcoin::secp256k1::schnorr::Signature> {
        let url = base.join(ep).expect("swap commit relative path");
        let request = wire_swap::SwapCommitmentRequest {
            inputs,
            outputs,
            expiry,
            wallet_key: wallet_pk,
        };
        let response: wire_swap::SwapCommitmentResponse = cl.post(url, &request).await?;
        crate::core::signature::schnorr_verify_b64(
            &response.content,
            &response.commitment,
            &mint_pk.x_only_public_key().0,
        )?;
        let expected_content = crate::core::signature::serialize_borsh_msg_b64(&request)?;
        if expected_content != response.content {
            return Err(Error::InvalidRequest(String::from(
                "content mismatch in commitment response",
            )));
        }
        Ok(response.commitment)
    }
}
