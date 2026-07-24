// ----- standard library imports
// ----- extra library imports
use bitcoin::{Amount, secp256k1};
use thiserror::Error;
use uuid::Uuid;
// ----- local imports
use crate::{
    cashu,
    client::admin::jsonrpc,
    core::BillId,
    wire::{exchange as wire_exchange, keys as wire_keys, treasury as wire_treasury},
};

// ----- end imports

pub mod admin_ep {
    pub const EBILL_MINTOP_STATUS: &str = "/admin/ebill/mintop/{qid}";
    pub const LIST_EBILL_MINTOPS: &str = "/admin/ebill/mintops/{kid}";
    pub const NEW_EBILL_MINTOP: &str = "/admin/ebill/mintop";
    pub const REQUEST_TO_PAY_EBILL: &str = "/admin/request_to_pay_ebill";
    pub const TRY_HTLC_SWAP: &str = "/admin/try_htlc_swap";
    pub const FEES_STORE_PROOFS: &str = "/admin/fees/store_proofs";
    pub const FEES_TOKEN: &str = "/admin/fees/token";
    pub const DENIED_MELTOPS: &str = "/admin/onchain/melt/denied";
    pub const DENIED_MELTOP: &str = "/admin/onchain/melt/denied/{qid}";
}

pub mod web_ep {
    pub const EBILLMINT_V1: &str = "/v1/mint/ebill";
    pub const EBILLMINT_V1_EXT: &str = "/v1/treasury/mint/ebill";
    pub const EXCHANGE_OFFLINE_V1: &str = "/v1/exchange/offline";
    pub const EXCHANGE_OFFLINE_V1_EXT: &str = "/v1/treasury/exchange/offline";
    pub const EXCHANGE_ONLINE_V1: &str = "/v1/exchange/online";
    pub const EXCHANGE_ONLINE_V1_EXT: &str = "/v1/treasury/exchange/online";
    pub const MELTQUOTE_ONCHAIN_V1: &str = "/v1/melt/onchain/quote";
    pub const MELTQUOTE_ONCHAIN_V1_EXT: &str = "/v1/treasury/melt/onchain/quote";
    pub const MELT_ONCHAIN_V1: &str = "/v1/melt/onchain";
    pub const MELT_ONCHAIN_V1_EXT: &str = "/v1/treasury/melt/onchain";
    pub const MELT_ONCHAIN_CONFIG_V1: &str = "/v1/melt/onchain/config";
    pub const MELT_ONCHAIN_CONFIG_V1_EXT: &str = "/v1/treasury/melt/onchain/config";
    pub const MELT_ONCHAIN_ESTIMATE_V1: &str = "/v1/melt/onchain/estimate";
    pub const MELT_ONCHAIN_ESTIMATE_V1_EXT: &str = "/v1/treasury/melt/onchain/estimate";
    pub const MINTQUOTE_ONCHAIN_V1: &str = "/v1/mint/onchain/quote";
    pub const MINTQUOTE_ONCHAIN_V1_EXT: &str = "/v1/treasury/mint/onchain/quote";
    pub const MINT_ONCHAIN_V1: &str = "/v1/mint/onchain";
    pub const MINT_ONCHAIN_V1_EXT: &str = "/v1/treasury/mint/onchain";
}

#[cfg(not(target_arch = "wasm32"))]
const CACHED_EPS: [(&str, reqwest::Method); 3] = [
    (web_ep::EBILLMINT_V1, reqwest::Method::POST),
    (web_ep::MELTQUOTE_ONCHAIN_V1, reqwest::Method::POST),
    (web_ep::MELT_ONCHAIN_V1, reqwest::Method::POST),
];

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("resource not found {0}")]
    ResourceNotFound(serde_json::Value),
    #[error("invalid request {0}")]
    InvalidRequest(serde_json::Value),
    #[error("service unavailable {0}")]
    ServiceUnavailable(SUError),
    #[error("internal {0}")]
    Internal(String),
    #[error("internal error {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("sign error {0}")]
    NUT20(#[from] cashu::nut20::Error),
}

/// service unavailable error
#[derive(Debug, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum SUError {
    #[error("unknown")]
    Unknown,
    #[error("melt operation temporarily suspended: {0}")]
    MeltOpSuspended(String),
}

impl std::convert::From<jsonrpc::Error> for Error {
    fn from(value: jsonrpc::Error) -> Self {
        match value {
            jsonrpc::Error::ResourceNotFound(msg) => Self::ResourceNotFound(msg),
            jsonrpc::Error::InvalidRequest(msg) => Self::InvalidRequest(msg),
            jsonrpc::Error::ServiceUnavailable(msg) => {
                let err: SUError = match serde_json::from_value(msg) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!("failed to deserialize SUError, {e}");
                        SUError::Unknown
                    }
                };
                Self::ServiceUnavailable(err)
            }
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

    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_retry(base: reqwest::Url, max_attempts: u32) -> Self {
        let builder = jsonrpc::retry::build_builder(&base, &CACHED_EPS, max_attempts);
        Self {
            cl: jsonrpc::Client::with_retry(builder),
            base,
        }
    }

    pub async fn request_to_pay_ebill(
        &self,
        ebill_id: BillId,
        amount: Amount,
        deadline: chrono::DateTime<chrono::Utc>,
    ) -> Result<wire_treasury::RequestToPayFromEBillResponse> {
        let request = wire_treasury::RequestToPayFromEBillRequest {
            ebill_id,
            amount,
            deadline,
        };
        let url = self
            .base
            .join(admin_ep::REQUEST_TO_PAY_EBILL)
            .expect("request_to_pay_ebill relative path");
        let response: wire_treasury::RequestToPayFromEBillResponse =
            self.cl.post(url, &request).await?;
        Ok(response)
    }

    pub async fn try_htlc(&self, preimage: String) -> Result<cashu::Amount> {
        let url = self
            .base
            .join(admin_ep::TRY_HTLC_SWAP)
            .expect("try_htlc relative path");
        let msg = wire_exchange::HtlcSwapAttemptRequest { preimage };
        let response = self.cl.post(url, &msg).await?;
        Ok(response)
    }

    pub async fn new_ebill_mint_operation(
        &self,
        qid: uuid::Uuid,
        kid: cashu::Id,
        pk: cashu::PublicKey,
        target: cashu::Amount,
        bill_id: BillId,
    ) -> Result<()> {
        let url = self
            .base
            .join(admin_ep::NEW_EBILL_MINTOP)
            .expect("ebill mint operation relative path");
        let msg = wire_treasury::NewMintOperationRequest {
            quote_id: qid,
            kid,
            pub_key: pk,
            target,
            bill_id,
        };
        let _: wire_treasury::NewMintOperationResponse = self.cl.post(url, &msg).await?;
        Ok(())
    }

    pub async fn ebill_mint_operation_status(
        &self,
        qid: uuid::Uuid,
    ) -> Result<wire_treasury::MintOperationStatus> {
        assert!(admin_ep::EBILL_MINTOP_STATUS.contains("{qid}"));
        let url = self
            .base
            .join(&admin_ep::EBILL_MINTOP_STATUS.replace("{qid}", &qid.to_string()))
            .expect("ebill mint operation status relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn list_ebill_mint_operations(&self, kid: cashu::Id) -> Result<Vec<uuid::Uuid>> {
        assert!(admin_ep::LIST_EBILL_MINTOPS.contains("{kid}"));
        let url = self
            .base
            .join(&admin_ep::LIST_EBILL_MINTOPS.replace("{kid}", &kid.to_string()))
            .expect("list ebill mint operations relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn exchange_offline_raw(
        &self,
        fingerprints: Vec<wire_keys::ProofFingerprint>,
        hashes: Vec<bitcoin::hashes::sha256::Hash>,
        wallet_pk: cashu::PublicKey,
    ) -> Result<wire_exchange::OfflineExchangeResponse> {
        let result = common::exchange_offline_raw(
            &self.cl,
            &self.base,
            web_ep::EXCHANGE_OFFLINE_V1,
            fingerprints,
            hashes,
            wallet_pk,
        )
        .await?;
        Ok(result)
    }

    pub async fn exchange_online(
        &self,
        proofs: Vec<cashu::Proof>,
        exchange_path: Vec<secp256k1::PublicKey>,
    ) -> Result<Vec<cashu::Proof>> {
        let result = common::exchange_online_raw(
            &self.cl,
            &self.base,
            web_ep::EXCHANGE_ONLINE_V1,
            proofs,
            exchange_path,
        )
        .await?;
        Ok(result.proofs)
    }

    pub async fn fees_store_proofs(&self, proofs: Vec<cashu::Proof>) -> Result<()> {
        let url = self
            .base
            .join(admin_ep::FEES_STORE_PROOFS)
            .expect("fees store proofs relative path");
        let msg = wire_treasury::StoreProofsRequest { proofs };
        let _: wire_treasury::StoreProofsResponse = self.cl.post(url, &msg).await?;
        Ok(())
    }

    pub async fn fees_token(&self) -> Result<wire_treasury::FeesTokenResponse> {
        let url = self
            .base
            .join(admin_ep::FEES_TOKEN)
            .expect("fees token relative path");
        let response: wire_treasury::FeesTokenResponse = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn list_denied(&self) -> Result<Vec<wire_treasury::DeniedMeltOp>> {
        let url = self
            .base
            .join(admin_ep::DENIED_MELTOPS)
            .expect("denied melt operations relative path");
        let response: wire_treasury::DeniedMeltOperations = self.cl.get(url, &[]).await?;
        Ok(response.ops)
    }

    pub async fn delete_denied(&self, id: Uuid) -> Result<()> {
        assert!(admin_ep::DENIED_MELTOP.contains("{qid}"));
        let ep = admin_ep::DENIED_MELTOP.replace("{qid}", &id.to_string());
        let url = self
            .base
            .join(&ep)
            .expect("denied melt operations relative path");
        self.cl.delete(url, &[]).await?;
        Ok(())
    }
}

pub(crate) mod common {
    use super::*;

    pub async fn exchange_offline_raw(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        fingerprints: Vec<wire_keys::ProofFingerprint>,
        hashes: Vec<bitcoin::hashes::sha256::Hash>,
        wallet_pk: cashu::PublicKey,
    ) -> Result<wire_exchange::OfflineExchangeResponse> {
        let url = base.join(ep).expect("exchange_offline relative path");
        let msg = wire_exchange::OfflineExchangeRequest {
            fingerprints,
            hashes,
            wallet_pk,
        };
        let response: wire_exchange::OfflineExchangeResponse = cl.post(url, &msg).await?;
        Ok(response)
    }

    pub async fn exchange_online_raw(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        proofs: Vec<cashu::Proof>,
        exchange_path: Vec<secp256k1::PublicKey>,
    ) -> Result<wire_exchange::OnlineExchangeResponse> {
        let url = base.join(ep).expect("exchange_online relative path");
        let msg = wire_exchange::OnlineExchangeRequest {
            proofs,
            exchange_path,
        };
        let response: wire_exchange::OnlineExchangeResponse = cl.post(url, &msg).await?;
        Ok(response)
    }
}
