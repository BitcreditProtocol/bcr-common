// ----- standard library imports
// ----- extra library imports
use bitcoin::{
    Amount, hashes::sha256::Hash as Sha256Hash, secp256k1, secp256k1::schnorr::Signature,
};
use thiserror::Error;
// ----- local imports
use crate::{
    cashu,
    core::{BillId, signature},
    wire::{exchange as wire_exchange, keys as wire_keys, treasury as wire_treasury},
};

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Error)]
pub enum Error {
    #[error("resource not found {0}")]
    ResourceNotFound(String),
    #[error("signature verification {0}")]
    Signature(#[from] signature::BorshMsgSignatureError),
    #[error("mint operation not found {0}")]
    MintOpNotFound(uuid::Uuid),

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

    pub const REQTOPAY_EP_V1: &str = "/v1/admin/treasury/request_to_pay_ebill";
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
            .join(Self::REQTOPAY_EP_V1)
            .expect("request_to_pay_ebill relative path");
        let request = self.cl.post(url).json(&request);
        let response: wire_treasury::RequestToPayFromEBillResponse =
            request.send().await?.json().await?;
        Ok(response)
    }

    pub const EXCHANGEONLINE_EP_V1: &'static str = "/v1/treasury/exchange/online";
    pub async fn exchange_online_raw(
        &self,
        proofs: Vec<cashu::Proof>,
        exchange_path: Vec<secp256k1::PublicKey>,
    ) -> Result<wire_exchange::OnlineExchangeResponse> {
        let url = self
            .base
            .join(Self::EXCHANGEONLINE_EP_V1)
            .expect("exchange_online relative path");
        let msg = wire_exchange::OnlineExchangeRequest {
            proofs,
            exchange_path,
        };
        let request = self.cl.post(url).json(&msg);
        let response: wire_exchange::OnlineExchangeResponse = request.send().await?.json().await?;
        Ok(response)
    }

    pub async fn exchange_online(
        &self,
        proofs: Vec<cashu::Proof>,
        exchange_path: Vec<secp256k1::PublicKey>,
    ) -> Result<Vec<cashu::Proof>> {
        let response = self.exchange_online_raw(proofs, exchange_path).await?;
        Ok(response.proofs)
    }

    pub const EXCHANGEOFFLINE_EP_V1: &'static str = "/v1/treasury/exchange/offline";
    pub async fn exchange_offline_raw(
        &self,
        fingerprints: Vec<wire_keys::ProofFingerprint>,
        hashes: Vec<Sha256Hash>,
        wallet_pk: cashu::PublicKey,
    ) -> Result<wire_exchange::OfflineExchangeResponse> {
        let url = self
            .base
            .join(Self::EXCHANGEOFFLINE_EP_V1)
            .expect("exchange_offline relative path");
        let msg = wire_exchange::OfflineExchangeRequest {
            fingerprints,
            hashes,
            wallet_pk,
        };
        let request = self.cl.post(url).json(&msg);
        let response: wire_exchange::OfflineExchangeResponse = request.send().await?.json().await?;
        Ok(response)
    }

    pub async fn exchange_offline(
        &self,
        fingerprints: Vec<wire_keys::ProofFingerprint>,
        hashes: Vec<Sha256Hash>,
        wallet_pk: cashu::PublicKey,
        mint_pk: secp256k1::PublicKey,
    ) -> Result<(Vec<cashu::Proof>, Signature)> {
        let response = self
            .exchange_offline_raw(fingerprints, hashes, wallet_pk)
            .await?;
        signature::schnorr_verify_b64(
            &response.content,
            &response.signature,
            &mint_pk.x_only_public_key().0,
        )?;
        let payload: wire_exchange::OfflineExchangePayload =
            signature::deserialize_borsh_msg(&response.content)?;
        Ok((payload.proofs, response.signature))
    }

    pub const TRYHTLC_EP_V1: &'static str = "/v1/admin/treasury/try_htlc_swap";
    pub async fn try_htlc(&self, preimage: String) -> Result<cashu::Amount> {
        let url = self
            .base
            .join(Self::TRYHTLC_EP_V1)
            .expect("try_htlc relative path");
        let msg = wire_exchange::HtlcSwapAttemptRequest { preimage };
        let request = self.cl.post(url).json(&msg);
        let response = request.send().await?.json().await?;
        Ok(response)
    }

    pub const NEWEBILLMINTOP_EP_V1: &'static str = "/v1/admin/treasury/ebill/mintop";
    pub async fn new_ebill_mint_operation(
        &self,
        qid: uuid::Uuid,
        kid: cashu::Id,
        pk: cashu::PublicKey,
        target: cashu::Amount,
        bill_id: crate::core::BillId,
    ) -> Result<()> {
        let url = self
            .base
            .join(Self::NEWEBILLMINTOP_EP_V1)
            .expect("ebill mint operation relative path");
        let msg = wire_treasury::NewMintOperationRequest {
            quote_id: qid,
            kid,
            pub_key: pk,
            target,
            bill_id,
        };
        let request = self.cl.post(url).json(&msg);
        let _ = request
            .send()
            .await?
            .json::<wire_treasury::NewMintOperationResponse>()
            .await?;
        Ok(())
    }

    pub const EBILLMINTOPSTATUS_EP_V1: &'static str = "/v1/admin/treasury/ebill/mintop/{qid}";
    pub async fn ebill_mint_operation_status(
        &self,
        qid: uuid::Uuid,
    ) -> Result<wire_treasury::MintOperationStatus> {
        let url = self
            .base
            .join(&Self::EBILLMINTOPSTATUS_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("ebill mint operation status relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::MintOpNotFound(qid));
        }
        let response = response
            .json::<wire_treasury::MintOperationStatus>()
            .await?;
        Ok(response)
    }

    pub const LISTEBILLMINTOPS_EP_V1: &'static str = "/v1/admin/treasury/ebill/mintops/{kid}";
    pub async fn list_ebill_mint_operations(&self, kid: cashu::Id) -> Result<Vec<uuid::Uuid>> {
        let url = self
            .base
            .join(&Self::LISTEBILLMINTOPS_EP_V1.replace("{kid}", &kid.to_string()))
            .expect("list ebill mint operations relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        let response = response.json::<Vec<uuid::Uuid>>().await?;
        Ok(response)
    }

    pub const EBILLMINT_EP_V1: &'static str = "/v1/mint/ebill";
    pub async fn ebill_mint(
        &self,
        qid: uuid::Uuid,
        outputs: Vec<cashu::BlindedMessage>,
        sk: cashu::SecretKey,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let url = self
            .base
            .join(Self::EBILLMINT_EP_V1)
            .expect("ebill mint relative path");
        let mut msg = cashu::MintRequest {
            quote: qid,
            outputs,
            signature: None,
        };
        msg.sign(sk)?;
        let result = self.cl.post(url).json(&msg).send().await?;
        if result.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::MintOpNotFound(qid));
        }
        let response = result.json::<cashu::MintResponse>().await?;
        Ok(response.signatures)
    }

    pub const MELTQUOTE_ONCHAIN_EP_V1: &'static str = "/v1/melt/onchain/quote";
    pub const MINTQUOTE_ONCHAIN_EP_V1: &'static str = "/v1/mint/onchain/quote";
    pub const MELT_ONCHAIN_EP_V1: &'static str = "/v1/melt/onchain";
    pub const MINT_ONCHAIN_EP_V1: &'static str = "/v1/mint/onchain";
}
