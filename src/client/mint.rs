// ----- standard library imports
// ----- extra library imports
use bitcoin::secp256k1;
use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;
// ----- local imports
use crate::{
    cashu,
    client::{core, treasury},
    core::signature::{self, BorshMsgSignatureError},
    wire::{
        clowder as wire_clowder, exchange as wire_exchange, keys as wire_keys, melt as wire_melt,
        mint as wire_mint, quotes as wire_quotes,
    },
};

// ----- end imports

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("resource not found {0}")]
    KeysetIdNotFound(cashu::Id),
    #[error("resource not found {0}")]
    ResourceNotFound(String),
    #[error("invalid request {0}")]
    InvalidRequest(String),
    #[error("internal {0}")]
    Internal(String),
    #[error("unimplemented")]
    Todo,

    #[error("signature {0}")]
    Signature(#[from] BorshMsgSignatureError),
    #[error("cdk::nut20 {0}")]
    Cdk20(#[from] cashu::nut20::Error),
    #[error("internal error {0}")]
    Reqwest(#[from] reqwest::Error),
}

impl std::convert::From<core::Error> for Error {
    fn from(value: core::Error) -> Self {
        match value {
            core::Error::KeysetIdNotFound(kid) => Error::KeysetIdNotFound(kid),
            core::Error::InvalidRequest(e) => Error::InvalidRequest(e),
            core::Error::Reqwest(e) => Error::Reqwest(e),
            core::Error::NUT20(e) => Error::Cdk20(e),
        }
    }
}

impl std::convert::From<treasury::Error> for Error {
    fn from(value: treasury::Error) -> Self {
        match value {
            treasury::Error::MintOpNotFound(s) => Error::ResourceNotFound(s.to_string()),
            treasury::Error::Reqwest(e) => Error::Reqwest(e),
            treasury::Error::NUT20(e) => Error::Cdk20(e),
        }
    }
}

/// A single public-facing client that covers the publicly available APIs
/// across the core, quote, and treasury services.
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

    // -------------------------------------------------------------------------
    // Core service – key / keyset endpoints
    // -------------------------------------------------------------------------

    pub const KEYS_EP_V1: &str = core::common::KEYS_EP_V1;
    pub async fn keys(&self, kid: cashu::Id) -> Result<cashu::KeySet> {
        let result = core::common::keys(&self.cl, &self.base, kid).await?;
        Ok(result)
    }

    pub const KEYSETINFO_EP_V1: &str = core::common::KEYSETINFO_EP_V1;
    pub async fn keyset_info(&self, kid: cashu::Id) -> Result<cashu::KeySetInfo> {
        let result = core::common::keyset_info(&self.cl, &self.base, kid).await?;
        Ok(result)
    }

    pub const LISTKEYSETINFO_EP_V1: &str = core::common::LISTKEYSETINFO_EP_V1;
    pub async fn list_keyset_info(
        &self,
        filters: wire_keys::KeysetInfoFilters,
    ) -> Result<Vec<cashu::KeySetInfo>> {
        let result = core::common::list_keyset_info(&self.cl, &self.base, filters).await?;
        Ok(result)
    }

    // -------------------------------------------------------------------------
    // Core service – swap / burn / restore / check-state endpoints
    // -------------------------------------------------------------------------

    pub const SWAP_EP_V1: &str = core::common::SWAP_EP_V1;
    pub async fn swap(
        &self,
        inputs: Vec<cashu::Proof>,
        outputs: Vec<cashu::BlindedMessage>,
        commitment: bitcoin::secp256k1::schnorr::Signature,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let result = core::common::swap(&self.cl, &self.base, inputs, outputs, commitment).await?;
        Ok(result)
    }

    pub const RESTORE_EP_V1: &str = "/v1/core/restore";
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
        let ret_val = outputs.into_iter().zip(signatures).collect::<Vec<_>>();
        Ok(ret_val)
    }

    pub const CHECKSTATE_EP_V1: &str = core::common::CHECKSTATE_EP_V1;
    pub async fn check_state(&self, ys: Vec<cashu::PublicKey>) -> Result<Vec<cashu::ProofState>> {
        let result = core::common::check_state(&self.cl, &self.base, ys).await?;
        Ok(result)
    }

    // -------------------------------------------------------------------------
    // Quote service – public endpoints
    // -------------------------------------------------------------------------

    pub const ENQUIRE_EP_V1: &str = "/v1/quote/ebill";
    pub async fn enquire(
        &self,
        bill: wire_quotes::SharedBill,
        minting_pubkey: cashu::PublicKey,
        signing_key: &bitcoin::secp256k1::Keypair,
    ) -> Result<Uuid> {
        let request = wire_quotes::EnquireRequest {
            content: bill,
            minting_pubkey,
        };
        let (content, sig) = signature::serialize_n_schnorr_sign_borsh_msg(&request, signing_key)?;
        let signed = wire_quotes::SignedEnquireRequest {
            content,
            signature: sig,
        };
        let url = self
            .base
            .join(Self::ENQUIRE_EP_V1)
            .expect("enquire relative path");
        let response = self.cl.post(url).json(&signed).send().await?;
        let reply = response.json::<wire_quotes::EnquireReply>().await?;
        Ok(reply.id)
    }

    pub const LOOKUP_EP_V1: &str = "/v1/quote/ebill/{qid}";
    pub async fn lookup(&self, qid: Uuid) -> Result<wire_quotes::StatusReply> {
        let url = self
            .base
            .join(&Self::LOOKUP_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("lookup relative path");
        let response = self.cl.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(qid.to_string()));
        }
        let reply = response.json::<wire_quotes::StatusReply>().await?;
        Ok(reply)
    }

    pub const RESOLVE_EP_V1: &str = "/v1/quote/ebill/{qid}";
    pub async fn accept_offer(&self, qid: Uuid) -> Result<()> {
        let url = self
            .base
            .join(&Self::RESOLVE_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("accept offer relative path");
        let response = self
            .cl
            .patch(url)
            .json(&wire_quotes::ResolveOffer::Accept)
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(qid.to_string()));
        }
        Ok(())
    }

    pub async fn reject_offer(&self, qid: Uuid) -> Result<()> {
        let url = self
            .base
            .join(&Self::RESOLVE_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("reject offer relative path");
        let response = self
            .cl
            .patch(url)
            .json(&wire_quotes::ResolveOffer::Reject)
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(qid.to_string()));
        }
        Ok(())
    }

    pub async fn cancel_enquiry(&self, qid: Uuid) -> Result<()> {
        let url = self
            .base
            .join(&Self::RESOLVE_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("cancel enquiry relative path");
        let response = self.cl.delete(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(qid.to_string()));
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Treasury service – public endpoints
    // -------------------------------------------------------------------------

    pub const EXCHANGEONLINE_EP_V1: &str = treasury::common::EXCHANGEONLINE_EP_V1;
    pub async fn exchange_online_raw(
        &self,
        proofs: Vec<cashu::Proof>,
        exchange_path: Vec<secp256k1::PublicKey>,
    ) -> Result<wire_exchange::OnlineExchangeResponse> {
        let result =
            treasury::common::exchange_online_raw(&self.cl, &self.base, proofs, exchange_path)
                .await?;
        Ok(result)
    }

    pub async fn exchange_online(
        &self,
        proofs: Vec<cashu::Proof>,
        exchange_path: Vec<secp256k1::PublicKey>,
    ) -> Result<Vec<cashu::Proof>> {
        let response = self.exchange_online_raw(proofs, exchange_path).await?;
        Ok(response.proofs)
    }

    pub const EXCHANGEOFFLINE_EP_V1: &str = treasury::common::EXCHANGEOFFLINE_EP_V1;
    pub async fn exchange_offline_raw(
        &self,
        fingerprints: Vec<wire_keys::ProofFingerprint>,
        hashes: Vec<bitcoin::hashes::sha256::Hash>,
        wallet_pk: cashu::PublicKey,
    ) -> Result<wire_exchange::OfflineExchangeResponse> {
        let result = treasury::common::exchange_offline_raw(
            &self.cl,
            &self.base,
            fingerprints,
            hashes,
            wallet_pk,
        )
        .await?;
        Ok(result)
    }

    pub async fn exchange_offline(
        &self,
        fingerprints: Vec<wire_keys::ProofFingerprint>,
        hashes: Vec<bitcoin::hashes::sha256::Hash>,
        wallet_pk: cashu::PublicKey,
        mint_pk: secp256k1::PublicKey,
    ) -> Result<(Vec<cashu::Proof>, secp256k1::schnorr::Signature)> {
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

    pub const EBILLMINT_EP_V1: &str = "/v1/treasury/mint/ebill";
    pub async fn ebill_mint(
        &self,
        qid: Uuid,
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
            return Err(Error::ResourceNotFound(qid.to_string()));
        }
        let response = result.json::<cashu::MintResponse>().await?;
        Ok(response.signatures)
    }

    /// target: the amount you expect to receive in recipient
    pub const MELTQUOTE_ONCHAIN_EP_V1: &str = "/v1/treasury/melt/onchain/quote";
    pub async fn onchain_melt_quote(
        &self,
        recipient: bitcoin::Address,
        target: bitcoin::Amount,
        change: Vec<cashu::BlindedMessage>,
    ) -> Result<(Uuid, cashu::Amount, DateTime<Utc>)> {
        let url = self
            .base
            .join(Self::MELTQUOTE_ONCHAIN_EP_V1)
            .expect("onchain melt quote relative path");
        let invoice = wire_melt::OnchainInvoice {
            address: recipient.into_unchecked(),
            amount: target,
        };
        let msg = wire_melt::MeltQuoteOnchainRequest {
            unit: crate::client::CURRENCY_UNIT,
            request: invoice,
            change,
        };
        let request = self.cl.post(url).json(&msg);
        let response: wire_melt::MeltQuoteOnchainResponse = request.send().await?.json().await?;
        let wire_melt::MeltQuoteOnchainResponse {
            quote,
            fee_reserve,
            amount,
            expiry,
            ..
        } = response;
        let total = amount + fee_reserve;
        let ctotal = cashu::Amount::from(total.to_sat());
        let expiration = DateTime::from_timestamp(expiry as i64, 0).ok_or(Error::Internal(
            format!("chrono::from_timestamp failed for {expiry}"),
        ))?;
        Ok((quote, ctotal, expiration))
    }

    pub const MINTQUOTE_ONCHAIN_EP_V1: &str = "/v1/treasury/mint/onchain/quote";
    pub async fn onchain_mint_quote(
        &self,
        blinds: Vec<cashu::BlindedMessage>,
        mint_pk: secp256k1::PublicKey,
    ) -> Result<wire_mint::OnchainMintQuoteResponse> {
        let url = self
            .base
            .join(Self::MINTQUOTE_ONCHAIN_EP_V1)
            .expect("onchain mint quote relative path");
        let msg = wire_mint::OnchainMintQuoteRequest {
            blinded_messages: blinds,
        };
        let request = self.cl.post(url).json(&msg);
        let response: wire_mint::OnchainMintQuoteResponse = request.send().await?.json().await?;
        signature::schnorr_verify_b64(
            &response.content,
            &response.commitment,
            &mint_pk.x_only_public_key().0,
        )?;
        Ok(response)
    }

    pub const MELT_ONCHAIN_EP_V1: &str = "/v1/treasury/melt/onchain";
    pub async fn onchain_melt(&self, _qid: Uuid, _inputs: Vec<cashu::Proof>) -> Result<()> {
        let _url = self
            .base
            .join(Self::MELT_ONCHAIN_EP_V1)
            .expect("onchain melt relative path");
        Err(Error::Todo)
    }

    pub const MINT_ONCHAIN_EP_V1: &str = "/v1/treasury/mint/onchain";
    pub async fn onchain_mint(&self, _qid: Uuid) -> Result<()> {
        let _url = self
            .base
            .join(Self::MINT_ONCHAIN_EP_V1)
            .expect("onchain mint relative path");
        Err(Error::Todo)
    }

    // -------------------------------------------------------------------------
    // Clowder service – public endpoints
    // -------------------------------------------------------------------------

    pub const FOREIGN_OFFLINE_EP_V1: &str = "/v1/clowder/foreign/offline/{alpha_id}";
    pub async fn get_offline(
        &self,
        alpha_id: secp256k1::PublicKey,
    ) -> Result<wire_clowder::OfflineResponse> {
        let url = self
            .base
            .join(&Self::FOREIGN_OFFLINE_EP_V1.replace("{alpha_id}", &alpha_id.to_string()))
            .expect("offline relative path");
        let response = self.cl.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(alpha_id.to_string()));
        }
        let payload = response.json().await?;
        Ok(payload)
    }

    pub const FOREIGN_STATUS_EP_V1: &str = "/v1/clowder/foreign/status/{alpha_id}";
    pub async fn get_status(
        &self,
        alpha_id: secp256k1::PublicKey,
    ) -> Result<wire_clowder::AlphaStateResponse> {
        let url = self
            .base
            .join(&Self::FOREIGN_STATUS_EP_V1.replace("{alpha_id}", &alpha_id.to_string()))
            .expect("status relative path");
        let response = self.cl.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(alpha_id.to_string()));
        }
        let response = response.json().await?;
        Ok(response)
    }

    pub const FOREIGN_SUBSTITUTE_EP_V1: &str = "/v1/clowder/foreign/substitute/{alpha_id}";
    pub async fn get_substitute(
        &self,
        alpha_id: secp256k1::PublicKey,
    ) -> Result<wire_clowder::ConnectedMintResponse> {
        let url = self
            .base
            .join(&Self::FOREIGN_SUBSTITUTE_EP_V1.replace("{alpha_id}", &alpha_id.to_string()))
            .expect("substitute relative path");
        let response = self.cl.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(alpha_id.to_string()));
        }
        let response = response.json().await?;
        Ok(response)
    }

    pub const FOREIGN_KEYSETS_EP_V1: &str = "/v1/clowder/foreign/keysets/{alpha_id}";
    pub async fn get_active_keysets(
        &self,
        alpha_id: secp256k1::PublicKey,
    ) -> Result<cashu::KeysResponse> {
        let url = self
            .base
            .join(&Self::FOREIGN_KEYSETS_EP_V1.replace("{alpha_id}", &alpha_id.to_string()))
            .expect("keysets relative path");
        let response = self.cl.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(alpha_id.to_string()));
        }
        let response = response.json().await?;
        Ok(response)
    }

    pub const LOCAL_PATH_EP_V1: &str = "/v1/clowder/local/path";
    pub async fn post_path(
        &self,
        origin_mint_url: reqwest::Url,
    ) -> Result<wire_clowder::ConnectedMintsResponse> {
        let url = self
            .base
            .join(Self::LOCAL_PATH_EP_V1)
            .expect("path relative path");
        let request = wire_clowder::PathRequest {
            origin_mint_url: origin_mint_url.clone(),
        };
        let response = self.cl.post(url).json(&request).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(origin_mint_url.to_string()));
        }
        let response = response.json().await?;
        Ok(response)
    }

    pub const LOCAL_INFO_EP_V1: &str = "/v1/clowder/local/info";
    pub async fn get_info(&self) -> Result<wire_clowder::ClowderNodeInfo> {
        let url = self
            .base
            .join(Self::LOCAL_INFO_EP_V1)
            .expect("info relative path");
        let response = self.cl.get(url).send().await?;
        let response = response.json().await?;
        Ok(response)
    }

    pub const LOCAL_BETAS_EP_V1: &str = "/v1/clowder/local/betas";
    pub async fn get_betas(&self) -> Result<wire_clowder::ConnectedMintsResponse> {
        let url = self
            .base
            .join(Self::LOCAL_BETAS_EP_V1)
            .expect("betas relative path");
        let response = self.cl.get(url).send().await?;
        let response = response.json().await?;
        Ok(response)
    }

    pub const LOCAL_COVERAGE_EP_V1: &str = "/v1/clowder/local/coverage";
    pub async fn get_coverage(&self) -> Result<wire_clowder::Coverage> {
        let url = self
            .base
            .join(Self::LOCAL_COVERAGE_EP_V1)
            .expect("coverage relative path");
        let response = self.cl.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(String::new()));
        }
        let response = response.json().await?;
        Ok(response)
    }

    pub const ONLINE_EXCHANGE_EP_V1: &str = "/v1/clowder/exchange/online";
    pub async fn post_online_exchange(
        &self,
        request: wire_exchange::OnlineExchangeRequest,
    ) -> Result<wire_exchange::OnlineExchangeResponse> {
        let url = self
            .base
            .join(Self::ONLINE_EXCHANGE_EP_V1)
            .expect("online exchange relative path");
        let response = self.cl.post(url).json(&request).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            let ys: Vec<String> = request
                .proofs
                .into_iter()
                .map(|f| f.y().unwrap().to_string())
                .collect();
            let resources = ys.join(", ");
            return Err(Error::ResourceNotFound(resources));
        }
        let response = response.json().await?;
        Ok(response)
    }

    pub const OFFLINE_EXCHANGE_EP_V1: &str = "/v1/clowder/exchange/offline";
    pub async fn post_offline_exchange(
        &self,
        request: wire_exchange::OfflineExchangeRequest,
    ) -> Result<wire_exchange::OfflineExchangeResponse> {
        let url = self
            .base
            .join(Self::OFFLINE_EXCHANGE_EP_V1)
            .expect("offline exchange relative path");
        let response = self.cl.post(url).json(&request).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            let ys: Vec<String> = request
                .fingerprints
                .into_iter()
                .map(|f| f.y.to_string())
                .collect();
            let resources = ys.join(", ");
            return Err(Error::ResourceNotFound(resources));
        }
        let response = response.json().await?;
        Ok(response)
    }

    pub const LOCAL_DERIVE_EBILL_PAYMENT_ADDRESS_EP_V1: &str =
        "/v1/clowder/local/derive_ebill_payment_address";
    pub async fn post_derive_ebill_payment_address(
        &self,
        request: wire_clowder::DeriveEbillPaymentAddressRequest,
    ) -> Result<wire_clowder::DeriveEbillPaymentAddressResponse> {
        let url = self
            .base
            .join(Self::LOCAL_DERIVE_EBILL_PAYMENT_ADDRESS_EP_V1)
            .expect("derive ebill address relative path");
        let response = self.cl.post(url).json(&request).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(request.bill_id.to_string()));
        }
        let response = response.json().await?;
        Ok(response)
    }
}
