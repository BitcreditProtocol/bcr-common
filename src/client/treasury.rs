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
    wire::{
        exchange as wire_exchange, keys as wire_keys, signatures as wire_signatures,
        swap as wire_swap, treasury as wire_treasury, wallet as wire_wallet,
    },
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
    pub const REDEEM_EP_V1: &'static str = "/v1/treasury/redeem";
    pub async fn redeem(
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
            .join(Self::REDEEM_EP_V1)
            .expect("redeem relative path");
        let request = self.cl.post(url).json(&msg);
        let response: wire_swap::SwapResponse = request.send().await?.json().await?;
        Ok(response.signatures)
    }

    pub const REQTOPAY_EP_V1: &str = "/v1/admin/treasury/debit/request_to_pay_ebill";
    pub async fn request_to_pay_ebill(
        &self,
        ebill_id: BillId,
        amount: Amount,
        deadline: chrono::DateTime<chrono::Utc>,
    ) -> Result<wire_signatures::RequestToMintFromEBillResponse> {
        let request = wire_signatures::RequestToMintFromEBillRequest {
            ebill_id,
            amount,
            deadline,
        };
        let url = self
            .base
            .join(Self::REQTOPAY_EP_V1)
            .expect("request_to_pay_ebill relative path");
        let request = self.cl.post(url).json(&request);
        let response: wire_signatures::RequestToMintFromEBillResponse =
            request.send().await?.json().await?;
        Ok(response)
    }

    pub const SATBALANCE_EP_V1: &'static str = "/v1/admin/treasury/debit/balance";
    pub async fn sat_balance(&self) -> Result<wire_wallet::ECashBalance> {
        let url = self
            .base
            .join(Self::SATBALANCE_EP_V1)
            .expect("sat balance relative path");
        let request = self.cl.get(url);
        let response: wire_wallet::ECashBalance = request.send().await?.json().await?;
        Ok(response)
    }

    pub const SATEXCHANGEONLINE_EP_V1: &'static str = "/v1/treasury/debit/exchange/online";
    pub async fn sat_exchange_online_raw(
        &self,
        proofs: Vec<cashu::Proof>,
        exchange_path: Vec<secp256k1::PublicKey>,
    ) -> Result<wire_exchange::OnlineExchangeResponse> {
        let url = self
            .base
            .join(Self::SATEXCHANGEONLINE_EP_V1)
            .expect("sat_exchange_online relative path");
        let msg = wire_exchange::OnlineExchangeRequest {
            proofs,
            exchange_path,
        };
        let request = self.cl.post(url).json(&msg);
        let response: wire_exchange::OnlineExchangeResponse = request.send().await?.json().await?;
        Ok(response)
    }

    pub async fn sat_exchange_online(
        &self,
        proofs: Vec<cashu::Proof>,
        exchange_path: Vec<secp256k1::PublicKey>,
    ) -> Result<Vec<cashu::Proof>> {
        let response = self.sat_exchange_online_raw(proofs, exchange_path).await?;
        Ok(response.proofs)
    }

    pub const CRSATEXCHANGEONLINE_EP_V1: &'static str = "/v1/treasury/credit/exchange/online";
    pub async fn crsat_exchange_online_raw(
        &self,
        proofs: Vec<cashu::Proof>,
        exchange_path: Vec<secp256k1::PublicKey>,
    ) -> Result<wire_exchange::OnlineExchangeResponse> {
        let url = self
            .base
            .join(Self::CRSATEXCHANGEONLINE_EP_V1)
            .expect("crsat_exchange_online relative path");
        let msg = wire_exchange::OnlineExchangeRequest {
            proofs,
            exchange_path,
        };
        let request = self.cl.post(url).json(&msg);
        let response: wire_exchange::OnlineExchangeResponse = request.send().await?.json().await?;
        Ok(response)
    }

    pub async fn crsat_exchange_online(
        &self,
        proofs: Vec<cashu::Proof>,
        exchange_path: Vec<secp256k1::PublicKey>,
    ) -> Result<Vec<cashu::Proof>> {
        let response = self
            .crsat_exchange_online_raw(proofs, exchange_path)
            .await?;
        Ok(response.proofs)
    }

    pub const SATEXCHANGEOFFLINE_EP_V1: &'static str = "/v1/treasury/debit/exchange/offline";
    pub async fn sat_exchange_offline_raw(
        &self,
        fingerprints: Vec<wire_keys::ProofFingerprint>,
        hashes: Vec<Sha256Hash>,
        wallet_pk: cashu::PublicKey,
    ) -> Result<wire_exchange::OfflineExchangeResponse> {
        let url = self
            .base
            .join(Self::SATEXCHANGEOFFLINE_EP_V1)
            .expect("sat_exchange_offline relative path");
        let msg = wire_exchange::OfflineExchangeRequest {
            fingerprints,
            hashes,
            wallet_pk,
        };
        let request = self.cl.post(url).json(&msg);
        let response: wire_exchange::OfflineExchangeResponse = request.send().await?.json().await?;
        Ok(response)
    }

    pub async fn sat_exchange_offline(
        &self,
        fingerprints: Vec<wire_keys::ProofFingerprint>,
        hashes: Vec<Sha256Hash>,
        wallet_pk: cashu::PublicKey,
        mint_pk: secp256k1::PublicKey,
    ) -> Result<(Vec<cashu::Proof>, Signature)> {
        let response = self
            .sat_exchange_offline_raw(fingerprints, hashes, wallet_pk)
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

    pub const CRSATEXCHANGEOFFLINE_EP_V1: &'static str = "/v1/treasury/credit/exchange/offline";
    pub async fn crsat_exchange_offline_raw(
        &self,
        fingerprints: Vec<wire_keys::ProofFingerprint>,
        hashes: Vec<Sha256Hash>,
        wallet_pk: cashu::PublicKey,
    ) -> Result<wire_exchange::OfflineExchangeResponse> {
        let url = self
            .base
            .join(Self::CRSATEXCHANGEOFFLINE_EP_V1)
            .expect("crsat_exchange_offline relative path");
        let msg = wire_exchange::OfflineExchangeRequest {
            fingerprints,
            hashes,
            wallet_pk,
        };
        let request = self.cl.post(url).json(&msg);
        let response: wire_exchange::OfflineExchangeResponse = request.send().await?.json().await?;
        Ok(response)
    }

    pub async fn crsat_exchange_offline(
        &self,
        fingerprints: Vec<wire_keys::ProofFingerprint>,
        hashes: Vec<Sha256Hash>,
        wallet_pk: cashu::PublicKey,
        mint_pk: secp256k1::PublicKey,
    ) -> Result<(Vec<cashu::Proof>, Signature)> {
        let response = self
            .crsat_exchange_offline_raw(fingerprints, hashes, wallet_pk)
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

    pub const TRYSATHTLC_EP_V1: &'static str = "/v1/admin/treasury/debit/try_htlc_swap";
    pub async fn try_sat_htlc(&self, preimage: String) -> Result<cashu::Amount> {
        let url = self
            .base
            .join(Self::TRYSATHTLC_EP_V1)
            .expect("try_sat_htlc relative path");
        let msg = wire_exchange::HtlcSwapAttemptRequest { preimage };
        let request = self.cl.post(url).json(&msg);
        let response = request.send().await?.json().await?;
        Ok(response)
    }

    pub const TRYCRSATHTLC_EP_V1: &'static str = "/v1/admin/treasury/credit/try_htlc_swap";
    pub async fn try_crsat_htlc(&self, preimage: String) -> Result<cashu::Amount> {
        let url = self
            .base
            .join(Self::TRYCRSATHTLC_EP_V1)
            .expect("try_crsat_htlc relative path");
        let msg = wire_exchange::HtlcSwapAttemptRequest { preimage };
        let request = self.cl.post(url).json(&msg);
        let response = request.send().await?.json().await?;
        Ok(response)
    }

    pub const IS_EBILL_MINT_COMPLETE_EP_V1: &'static str =
        "/v1/admin/treasury/debit/mint_complete/{ebill_id}";
    pub async fn is_ebill_mint_complete(&self, ebill_id: BillId) -> Result<bool> {
        let path = Self::IS_EBILL_MINT_COMPLETE_EP_V1.replace("{ebill_id}", &ebill_id.to_string());
        let url = self
            .base
            .join(&path)
            .expect("is_ebill_mint_complete relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        if matches!(response.status(), reqwest::StatusCode::NOT_FOUND) {
            return Err(Error::ResourceNotFound(ebill_id.to_string()));
        }
        let response: wire_wallet::EbillPaymentComplete = response.json().await?;
        Ok(response.complete)
    }

    pub const NEWMINTOP_EP_V1: &'static str = "/v1/admin/treasury/credit/mintop";
    pub async fn new_mint_operation(
        &self,
        qid: uuid::Uuid,
        kid: cashu::Id,
        pk: cashu::PublicKey,
        target: cashu::Amount,
        bill_id: crate::core::BillId,
    ) -> Result<()> {
        let url = self
            .base
            .join(Self::NEWMINTOP_EP_V1)
            .expect("mint operation relative path");
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

    pub const MINTOPSTATUS_EP_V1: &'static str = "/v1/admin/treasury/credit/mintop/{qid}";
    pub async fn mint_operation_status(
        &self,
        qid: uuid::Uuid,
    ) -> Result<wire_treasury::MintOperationStatus> {
        let url = self
            .base
            .join(&Self::MINTOPSTATUS_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("mint operation status relative path");
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

    pub const LISTMINTOPS_EP_V1: &'static str = "/v1/admin/treasury/credit/mintops/{kid}";
    pub async fn list_mint_operations(&self, kid: cashu::Id) -> Result<Vec<uuid::Uuid>> {
        let url = self
            .base
            .join(&Self::LISTMINTOPS_EP_V1.replace("{kid}", &kid.to_string()))
            .expect("list mint operations relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        let response = response.json::<Vec<uuid::Uuid>>().await?;
        Ok(response)
    }

    pub const MINT_EP_V1: &'static str = "/v1/mint/credit";
    pub async fn mint(
        &self,
        qid: uuid::Uuid,
        outputs: Vec<cashu::BlindedMessage>,
        sk: cashu::SecretKey,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let url = self
            .base
            .join(Self::MINT_EP_V1)
            .expect("mint relative path");
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

    pub const MELTQUOTE_ONCHAIN_EP_V1: &'static str = "/v1/melt/quote/onchain";
    pub const MELT_ONCHAIN_EP_V1: &'static str = "/v1/melt/onchain";
    pub const MINTQUOTE_ONCHAIN_EP_V1: &'static str = "/v1/mint/quote/onchain";
}
