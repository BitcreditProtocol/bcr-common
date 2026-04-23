// ----- standard library imports
// ----- extra library imports
use bitcoin::{Amount, secp256k1};
use thiserror::Error;
// ----- local imports
use crate::{
    cashu,
    client::admin::jsonrpc,
    core::BillId,
    wire::{exchange as wire_exchange, keys as wire_keys, treasury as wire_treasury},
};

// ----- end imports

pub mod admin_ep {
    pub const EBILL_MINTOP_STATUS_V1: &str = "/v1/admin/ebill/mintop/{qid}";
    pub const LIST_EBILL_MINTOPS_V1: &str = "/v1/admin/ebill/mintops/{kid}";
    pub const NEW_EBILL_MINTOP_V1: &str = "/v1/admin/ebill/mintop";
    pub const REQUEST_TO_PAY_EBILL_V1: &str = "/v1/admin/request_to_pay_ebill";
    pub const TRY_HTLC_SWAP_V1: &str = "/v1/admin/try_htlc_swap";
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
    pub const MINTQUOTE_ONCHAIN_V1: &str = "/v1/mint/onchain/quote";
    pub const MINTQUOTE_ONCHAIN_V1_EXT: &str = "/v1/treasury/mint/onchain/quote";
    pub const MINT_ONCHAIN_V1: &str = "/v1/mint/onchain";
    pub const MINT_ONCHAIN_V1_EXT: &str = "/v1/treasury/mint/onchain";
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
}

impl std::convert::From<jsonrpc::Error> for Error {
    fn from(value: jsonrpc::Error) -> Self {
        match value {
            jsonrpc::Error::ResourceNotFound(msg) => Self::ResourceNotFound(msg),
            jsonrpc::Error::InvalidRequest(msg) => Self::InvalidRequest(msg),
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
            .join(admin_ep::REQUEST_TO_PAY_EBILL_V1)
            .expect("request_to_pay_ebill relative path");
        let response: wire_treasury::RequestToPayFromEBillResponse =
            self.cl.post(url, &request).await?;
        Ok(response)
    }

    pub async fn try_htlc(&self, preimage: String) -> Result<cashu::Amount> {
        let url = self
            .base
            .join(admin_ep::TRY_HTLC_SWAP_V1)
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
            .join(admin_ep::NEW_EBILL_MINTOP_V1)
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
        assert!(admin_ep::EBILL_MINTOP_STATUS_V1.contains("{qid}"));
        let url = self
            .base
            .join(&admin_ep::EBILL_MINTOP_STATUS_V1.replace("{qid}", &qid.to_string()))
            .expect("ebill mint operation status relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn list_ebill_mint_operations(&self, kid: cashu::Id) -> Result<Vec<uuid::Uuid>> {
        assert!(admin_ep::LIST_EBILL_MINTOPS_V1.contains("{kid}"));
        let url = self
            .base
            .join(&admin_ep::LIST_EBILL_MINTOPS_V1.replace("{kid}", &kid.to_string()))
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
