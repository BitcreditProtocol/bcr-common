// ----- standard library imports
// ----- extra library imports
use bitcoin::secp256k1;
use thiserror::Error;
use uuid::Uuid;
// ----- local imports
use crate::{
    cashu,
    core::signature::{self, BorshMsgSignatureError},
    wire::{
        clowder as wire_clowder, exchange as wire_exchange, keys as wire_keys,
        quotes as wire_quotes, swap as wire_swap,
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
    #[error("invalid request")]
    InvalidRequest,
    #[error("signature {0}")]
    Signature(#[from] BorshMsgSignatureError),
    #[error("cdk::nut20 {0}")]
    Cdk20(#[from] cashu::nut20::Error),

    #[error("internal error {0}")]
    Reqwest(#[from] reqwest::Error),
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

    pub const KEYS_EP_V1: &'static str = "/v1/core/keys/{kid}";
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

    pub const KEYSETINFO_EP_V1: &'static str = "/v1/core/keysets/{kid}";
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

    pub const LISTKEYSETINFO_EP_V1: &'static str = "/v1/core/keysets";
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

    // -------------------------------------------------------------------------
    // Core service – swap / burn / restore / check-state endpoints
    // -------------------------------------------------------------------------

    pub const SWAP_EP_V1: &'static str = "/v1/core/swap";
    pub async fn swap(
        &self,
        inputs: Vec<cashu::Proof>,
        outputs: Vec<cashu::BlindedMessage>,
        commitment: bitcoin::secp256k1::schnorr::Signature,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let url = self
            .base
            .join(Self::SWAP_EP_V1)
            .expect("swap relative path");
        let request = wire_swap::SwapRequest {
            inputs,
            outputs,
            commitment,
        };
        let response = self.cl.post(url).json(&request).send().await?;
        let signatures: wire_swap::SwapResponse = response.json().await?;
        Ok(signatures.signatures)
    }

    pub const BURN_EP_V1: &'static str = "/v1/core/burn";
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

    pub const RESTORE_EP_V1: &'static str = "/v1/core/restore";
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

    pub const CHECKSTATE_EP_V1: &'static str = "/v1/core/checkstate";
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

    // -------------------------------------------------------------------------
    // Quote service – public endpoints
    // -------------------------------------------------------------------------

    pub const ENQUIRE_EP_V1: &'static str = "/v1/quote/ebill";
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

    pub const LOOKUP_EP_V1: &'static str = "/v1/quote/ebill/{qid}";
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

    pub const RESOLVE_EP_V1: &'static str = "/v1/quote/ebill/{qid}";
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
        hashes: Vec<bitcoin::hashes::sha256::Hash>,
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

    pub const EBILLMINT_EP_V1: &'static str = "/v1/treasury/mint/ebill";
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

    pub const MELTQUOTE_ONCHAIN_EP_V1: &'static str = "/v1/treasury/melt/onchain/quote";
    pub async fn onchain_melt_quote(&self) -> Result<()> {
        todo!();
    }

    pub const MINTQUOTE_ONCHAIN_EP_V1: &'static str = "/v1/treasury/mint/onchain/quote";
    pub async fn onchain_mint_quote(&self) -> Result<()> {
        todo!();
    }

    pub const MELT_ONCHAIN_EP_V1: &'static str = "/v1/treasury/melt/onchain";
    pub async fn onchain_melt(&self) -> Result<()> {
        todo!();
    }

    pub const MINT_ONCHAIN_EP_V1: &'static str = "/v1/treasury/mint/onchain";
    pub async fn onchain_mint(&self) -> Result<()> {
        todo!();
    }

    // -------------------------------------------------------------------------
    // Clowder service – public endpoints
    // -------------------------------------------------------------------------

    pub const FOREIGN_OFFLINE_EP_V1: &'static str = "/v1/clowder/foreign/offline/{alpha_id}";
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

    pub const FOREIGN_STATUS_EP_V1: &'static str = "/v1/clowder/foreign/status/{alpha_id}";
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

    pub const FOREIGN_SUBSTITUTE_EP_V1: &'static str = "/v1/clowder/foreign/substitute/{alpha_id}";
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

    pub const FOREIGN_KEYSETS_EP_V1: &'static str = "/v1/clowder/foreign/keysets/{alpha_id}";
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

    pub const LOCAL_PATH_EP_V1: &'static str = "/v1/clowder/local/path";
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

    pub const LOCAL_INFO_EP_V1: &'static str = "/v1/clowder/local/info";
    pub async fn get_info(&self) -> Result<wire_clowder::ClowderNodeInfo> {
        let url = self
            .base
            .join(Self::LOCAL_INFO_EP_V1)
            .expect("info relative path");
        let response = self.cl.get(url).send().await?;
        let response = response.json().await?;
        Ok(response)
    }

    pub const LOCAL_BETAS_EP_V1: &'static str = "/v1/clowder/local/betas";
    pub async fn get_betas(&self) -> Result<wire_clowder::ConnectedMintsResponse> {
        let url = self
            .base
            .join(Self::LOCAL_BETAS_EP_V1)
            .expect("betas relative path");
        let response = self.cl.get(url).send().await?;
        let response = response.json().await?;
        Ok(response)
    }

    pub const LOCAL_COVERAGE_EP_V1: &'static str = "/v1/clowder/local/coverage";
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

    pub const ONLINE_EXCHANGE_EP_V1: &'static str = "/v1/clowder/exchange/online";
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

    pub const OFFLINE_EXCHANGE_EP_V1: &'static str = "/v1/clowder/exchange/offline";
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

    pub const LOCAL_DERIVE_EBILL_PAYMENT_ADDRESS_EP_V1: &'static str =
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
