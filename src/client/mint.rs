// ----- standard library imports
// ----- extra library imports
use bitcoin::secp256k1 as secp;
use thiserror::Error;
use uuid::Uuid;
// ----- local imports
use crate::{
    cashu,
    client::{
        admin::{
            clowder::{self, web_ep},
            jsonrpc,
        },
        core, quote, treasury,
    },
    core::{
        BillId,
        signature::{self, BorshMsgSignatureError},
    },
    ecash,
    wire::{
        attestation as wire_attestation, clowder as wire_clowder,
        common::{ProtestOutcome, ProtestOutcomeError},
        exchange as wire_exchange, keys as wire_keys, melt as wire_melt, mint as wire_mint,
        quotes as wire_quotes, swap as wire_swap,
    },
};

// ----- end imports

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("resource not found {0}")]
    ResourceNotFound(RNFError),
    #[error("invalid request {0}")]
    InvalidRequest(BRError),
    #[error("service unavailable {0}")]
    ServiceUnavailable(SUError),
    #[error("internal {0}")]
    Internal(String),
    #[error("internal error {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("cdk::nut20 {0}")]
    Cdk20(#[from] cashu::nut20::Error),
    #[error("borsh sign error {0}")]
    BorshSign(#[from] BorshMsgSignatureError),
    #[error("protest outcome {0}")]
    ProtestOutcome(#[from] ProtestOutcomeError),
}

#[derive(Debug, Error)]
pub enum RNFError {
    #[error("unknown")]
    Unknown,
    //core
    #[error("keyset not found {0}")]
    KeysetId(cashu::Id),
    #[error("resource not found {0}")]
    Generic(String),
    //quote
    #[error("quote: {0}")]
    Quote(serde_json::Value),
    //treasury
    #[error("treasury: {0}")]
    Treasury(serde_json::Value),
    //clowder
    #[error("clowder: {0}")]
    Clowder(serde_json::Value),
}

#[derive(Debug, Error)]
pub enum BRError {
    #[error("unknown")]
    Unknown,
    //core
    #[error("commitment content does not match request")]
    CommitmentMismatch,
    #[error("{0}")]
    Generic(String),
    //quote
    #[error("quote: {0}")]
    Quote(serde_json::Value),
    //treasury
    #[error("treasury: {0}")]
    Treasury(serde_json::Value),
    //clowder
    #[error("clowder: {0}")]
    Clowder(serde_json::Value),
}

#[derive(Debug, Error)]
pub enum SUError {
    #[error("unknown")]
    Unknown,
    //core
    #[error("core: {0}")]
    Core(serde_json::Value),
    //quote
    #[error("quote: {0}")]
    Quote(serde_json::Value),
    //treasury
    #[error("melt operation temporarily suspended: {0}")]
    MeltOpSuspended(String),
    //clowder
    #[error("clowder: {0}")]
    Clowder(serde_json::Value),
}

impl std::convert::From<core::RNFError> for RNFError {
    fn from(value: core::RNFError) -> Self {
        match value {
            core::RNFError::Unknown => RNFError::Unknown,
            core::RNFError::KeysetId(kid) => RNFError::KeysetId(kid),
            core::RNFError::Generic(rsrc) => RNFError::Generic(rsrc),
        }
    }
}

impl std::convert::From<core::BRError> for BRError {
    fn from(value: core::BRError) -> Self {
        match value {
            core::BRError::Unknown => BRError::Unknown,
            core::BRError::CommitmentMismatch => BRError::CommitmentMismatch,
            core::BRError::Generic(s) => BRError::Generic(s),
        }
    }
}

impl std::convert::From<treasury::SUError> for SUError {
    fn from(value: treasury::SUError) -> Self {
        match value {
            treasury::SUError::Unknown => SUError::Unknown,
            treasury::SUError::MeltOpSuspended(s) => SUError::MeltOpSuspended(s),
        }
    }
}

impl std::convert::From<core::Error> for Error {
    fn from(value: core::Error) -> Self {
        match value {
            core::Error::ResourceNotFound(e) => Error::ResourceNotFound(e.into()),
            core::Error::InvalidRequest(e) => Error::InvalidRequest(e.into()),
            core::Error::ServiceUnavailable(e) => Error::ServiceUnavailable(SUError::Core(e)),
            core::Error::Internal(e) => Error::Internal(e),
            core::Error::Reqwest(e) => Error::Reqwest(e),
            core::Error::BorshSign(e) => Error::BorshSign(e),
        }
    }
}

impl std::convert::From<treasury::Error> for Error {
    fn from(value: treasury::Error) -> Self {
        match value {
            treasury::Error::ResourceNotFound(e) => Error::ResourceNotFound(RNFError::Treasury(e)),
            treasury::Error::InvalidRequest(e) => Error::InvalidRequest(BRError::Treasury(e)),
            treasury::Error::ServiceUnavailable(e) => Error::ServiceUnavailable(e.into()),
            treasury::Error::Internal(e) => Error::Internal(e),
            treasury::Error::Reqwest(e) => Error::Reqwest(e),
            treasury::Error::NUT20(e) => Error::Cdk20(e),
        }
    }
}

impl std::convert::From<clowder::Error> for Error {
    fn from(value: clowder::Error) -> Self {
        match value {
            clowder::Error::ResourceNotFound(e) => Error::ResourceNotFound(RNFError::Clowder(e)),
            clowder::Error::InvalidRequest(e) => Error::InvalidRequest(BRError::Clowder(e)),
            clowder::Error::ServiceUnavailable(e) => Error::ServiceUnavailable(SUError::Clowder(e)),
            clowder::Error::Internal(e) => Error::Internal(e),
            clowder::Error::Reqwest(e) => Error::Reqwest(e),
        }
    }
}

impl std::convert::From<quote::Error> for Error {
    fn from(value: quote::Error) -> Self {
        match value {
            quote::Error::ResourceNotFound(e) => Error::ResourceNotFound(RNFError::Quote(e)),
            quote::Error::InvalidRequest(e) => Error::InvalidRequest(BRError::Quote(e)),
            quote::Error::ServiceUnavailable(e) => Error::ServiceUnavailable(SUError::Quote(e)),
            quote::Error::Internal(e) => Error::Internal(e),
            quote::Error::Reqwest(e) => Error::Reqwest(e),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
const CACHED_EPS: [(&str, reqwest::Method); 6] = [
    (
        core::web_ep::SIGNED_SWAP_COMMIT_V1_EXT,
        reqwest::Method::POST,
    ),
    (core::web_ep::SWAP_COMMIT_V1_EXT, reqwest::Method::POST),
    (core::web_ep::SWAP_V1_EXT, reqwest::Method::POST),
    (treasury::web_ep::EBILLMINT_V1_EXT, reqwest::Method::POST),
    (
        treasury::web_ep::MELTQUOTE_ONCHAIN_V1_EXT,
        reqwest::Method::POST,
    ),
    (treasury::web_ep::MELT_ONCHAIN_V1_EXT, reqwest::Method::POST),
];

/// A single public-facing client that covers the publicly available APIs
/// across the core, quote, and treasury services.
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

    pub fn mint_url(&self) -> &reqwest::Url {
        &self.base
    }

    // -------------------------------------------------------------------------
    // Core service – key / keyset endpoints
    // -------------------------------------------------------------------------

    #[deprecated(note = "use `keys` instead")]
    pub async fn keys_v1(&self, kid: cashu::Id) -> Result<ecash::KeySet> {
        let result =
            core::common::keys_v1(&self.cl, &self.base, core::web_ep::KEYS_V1_EXT, kid).await?;
        Ok(result.try_into().expect("unit conversion issue"))
    }

    pub async fn keys(&self, kid: cashu::Id) -> Result<ecash::KeySet> {
        let result =
            core::common::keys(&self.cl, &self.base, core::web_ep::KEYS_V2_EXT, kid).await?;
        Ok(result)
    }

    pub async fn keyset_info(&self, kid: cashu::Id) -> Result<ecash::KeySetInfo> {
        let result =
            core::common::keyset_info(&self.cl, &self.base, core::web_ep::KEYSET_INFO_V1_EXT, kid)
                .await?;
        Ok(result)
    }

    pub async fn list_keyset_info(
        &self,
        filters: wire_keys::KeysetInfoFilters,
    ) -> Result<Vec<ecash::KeySetInfo>> {
        let result = core::common::list_keyset_info(
            &self.cl,
            &self.base,
            core::web_ep::LIST_KEYSET_INFO_V1_EXT,
            filters,
        )
        .await?;
        Ok(result)
    }

    // -------------------------------------------------------------------------
    // Core service – swap / burn / restore / check-state endpoints
    // -------------------------------------------------------------------------

    /// return (serialized wire_swap::SwapCommitmentRequest, signature)
    pub async fn commit_swap(
        &self,
        inputs: Vec<wire_keys::ProofFingerprint>,
        outputs: Vec<ecash::BlindedMessage>,
        expiry: u64,
        wallet_pk: secp::PublicKey,
        mint_pk: secp::PublicKey,
        attestation: crate::wire::attestation::IssuanceAttestation,
    ) -> Result<(String, secp::schnorr::Signature)> {
        let result = core::common::commit_swap(
            &self.cl,
            &self.base,
            core::web_ep::SWAP_COMMIT_V1_EXT,
            inputs,
            outputs,
            expiry,
            wallet_pk,
            mint_pk,
            attestation,
        )
        .await?;
        Ok(result)
    }

    #[cfg(feature = "mint")]
    pub fn prepare_swap_commitment_request(
        inputs: Vec<wire_keys::ProofFingerprint>,
        outputs: Vec<ecash::BlindedMessage>,
        expiry: u64,
        wallet_pk: secp::PublicKey,
        attestation: crate::wire::attestation::IssuanceAttestation,
    ) -> wire_swap::SwapCommitmentRequest {
        core::common::prepare_swap_commitment_request(
            inputs,
            outputs,
            expiry,
            wallet_pk,
            attestation,
        )
    }

    #[cfg(feature = "mint")]
    pub async fn commit_swap_with_signature(
        &self,
        payload: String,
        signature: secp::schnorr::Signature,
        mint_pk: secp::PublicKey,
    ) -> Result<(String, secp::schnorr::Signature)> {
        let original = signature::deserialize_borsh_msg(&payload)?;
        let request = wire_swap::SignedSwapCommitmentRequest { payload, signature };
        let url = self
            .base
            .join(core::web_ep::SIGNED_SWAP_COMMIT_V1_EXT)
            .expect("signed swap commit relative path");
        let response: wire_swap::SwapCommitmentResponse = self
            .cl
            .post(url, &request)
            .await
            .map_err(core::Error::from)?;
        let result = core::common::verify_commitment(original, response, &mint_pk)?;
        Ok(result)
    }

    pub async fn swap(
        &self,
        inputs: Vec<ecash::Proof>,
        outputs: Vec<ecash::BlindedMessage>,
        commitment: secp::schnorr::Signature,
    ) -> Result<Vec<ecash::BlindSignature>> {
        let result = core::common::swap(
            &self.cl,
            &self.base,
            core::web_ep::SWAP_V1_EXT,
            inputs,
            outputs,
            commitment,
        )
        .await?;
        Ok(result)
    }

    pub async fn restore(
        &self,
        outputs: Vec<cashu::BlindedMessage>,
    ) -> Result<Vec<(cashu::BlindedMessage, cashu::BlindSignature)>> {
        let url = self
            .base
            .join(core::web_ep::RESTORE_V1_EXT)
            .expect("restore relative path");
        let msg = cashu::RestoreRequest { outputs };
        let response: cashu::RestoreResponse =
            self.cl.post(url, &msg).await.map_err(core::Error::from)?;
        let cashu::RestoreResponse {
            outputs,
            signatures,
            ..
        } = response;
        let ret_val = outputs.into_iter().zip(signatures).collect::<Vec<_>>();
        Ok(ret_val)
    }

    pub async fn check_state(&self, ys: Vec<cashu::PublicKey>) -> Result<Vec<cashu::ProofState>> {
        let result =
            core::common::check_state(&self.cl, &self.base, core::web_ep::CHECK_STATE_V1_EXT, ys)
                .await?;
        Ok(result)
    }

    // -------------------------------------------------------------------------
    // Quote service – public endpoints
    // -------------------------------------------------------------------------

    pub async fn enquire(
        &self,
        bill: wire_quotes::SharedBill,
        minting_pubkey: cashu::PublicKey,
        signing_key: &secp::Keypair,
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
            .join(quote::web_ep::ENQUIRE_V1_EXT)
            .expect("enquire relative path");
        let response: wire_quotes::EnquireReply = self
            .cl
            .post(url, &signed)
            .await
            .map_err(quote::Error::from)?;
        Ok(response.id)
    }

    pub async fn lookup(&self, qid: Uuid) -> Result<wire_quotes::StatusReply> {
        assert!(quote::web_ep::LOOKUP_V1_EXT.contains("{qid}"));
        let url = self
            .base
            .join(&quote::web_ep::LOOKUP_V1_EXT.replace("{qid}", &qid.to_string()))
            .expect("lookup relative path");
        let response: wire_quotes::StatusReply =
            self.cl.get(url, &[]).await.map_err(quote::Error::from)?;
        Ok(response)
    }

    pub async fn accept_offer(&self, qid: Uuid) -> Result<()> {
        assert!(quote::web_ep::RESOLVE_V1_EXT.contains("{qid}"));
        let url = self
            .base
            .join(&quote::web_ep::RESOLVE_V1_EXT.replace("{qid}", &qid.to_string()))
            .expect("accept offer relative path");
        self.cl
            .patch_no_response(url, &wire_quotes::ResolveOffer::Accept)
            .await
            .map_err(quote::Error::from)?;
        Ok(())
    }

    pub async fn reject_offer(&self, qid: Uuid) -> Result<()> {
        assert!(quote::web_ep::RESOLVE_V1_EXT.contains("{qid}"));
        let url = self
            .base
            .join(&quote::web_ep::RESOLVE_V1_EXT.replace("{qid}", &qid.to_string()))
            .expect("reject offer relative path");
        self.cl
            .patch_no_response(url, &wire_quotes::ResolveOffer::Reject)
            .await
            .map_err(quote::Error::from)?;
        Ok(())
    }

    pub async fn cancel_enquiry(&self, qid: Uuid) -> Result<()> {
        assert!(quote::web_ep::RESOLVE_V1_EXT.contains("{qid}"));
        let url = self
            .base
            .join(&quote::web_ep::RESOLVE_V1_EXT.replace("{qid}", &qid.to_string()))
            .expect("cancel enquiry relative path");
        self.cl.delete(url, &[]).await.map_err(quote::Error::from)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Treasury service – public endpoints
    // -------------------------------------------------------------------------

    pub async fn exchange_online(
        &self,
        proofs: Vec<cashu::Proof>,
        exchange_path: Vec<secp::PublicKey>,
    ) -> Result<Vec<cashu::Proof>> {
        let response = treasury::common::exchange_online_raw(
            &self.cl,
            &self.base,
            treasury::web_ep::EXCHANGE_ONLINE_V1_EXT,
            proofs,
            exchange_path,
        )
        .await?;
        Ok(response.proofs)
    }

    pub async fn exchange_offline(
        &self,
        fingerprints: Vec<wire_keys::ProofFingerprint>,
        hashes: Vec<bitcoin::hashes::sha256::Hash>,
        wallet_pk: cashu::PublicKey,
        wallet_signature: secp::schnorr::Signature,
        mint_pk: secp::PublicKey,
    ) -> Result<(Vec<cashu::Proof>, secp::schnorr::Signature)> {
        let response = treasury::common::exchange_offline_raw(
            &self.cl,
            &self.base,
            treasury::web_ep::EXCHANGE_OFFLINE_V1_EXT,
            fingerprints,
            hashes,
            wallet_pk,
            wallet_signature,
        )
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

    pub async fn redeem_exchange_offline(
        &self,
        request: wire_exchange::RedeemOfflineExchangeRequest,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let response = treasury::common::redeem_exchange_offline(
            &self.cl,
            &self.base,
            treasury::web_ep::EXCHANGE_OFFLINE_REDEEM_V1_EXT,
            request,
        )
        .await?;
        Ok(response.signatures)
    }

    pub async fn ebill_mint(
        &self,
        qid: Uuid,
        outputs: Vec<cashu::BlindedMessage>,
        sk: cashu::SecretKey,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let url = self
            .base
            .join(treasury::web_ep::EBILLMINT_V1_EXT)
            .expect("ebill mint relative path");
        let mut msg = cashu::MintRequest {
            quote: qid,
            outputs,
            signature: None,
        };
        msg.sign(sk)?;
        let response: cashu::MintResponse = self
            .cl
            .post(url, &msg)
            .await
            .map_err(treasury::Error::from)?;
        Ok(response.signatures)
    }

    /// amount: the amount you expect to receive in recipient
    /// network_fee: total tx fee in sats you pay for the onchain transaction
    /// return: serialized (wire_melt::MeltQuoteOnchainResponseBody, signature)
    #[allow(clippy::too_many_arguments)]
    pub async fn onchain_melt_quote(
        &self,
        inputs: Vec<wire_keys::ProofFingerprint>,
        recipient: bitcoin::Address<bitcoin::address::NetworkUnchecked>,
        amount: bitcoin::Amount,
        network_fee: bitcoin::Amount,
        wallet_key: secp256k1::PublicKey,
        mint_pk: secp::PublicKey,
        attestation: crate::wire::attestation::IssuanceAttestation,
    ) -> Result<(String, secp::schnorr::Signature)> {
        let url = self
            .base
            .join(treasury::web_ep::MELTQUOTE_ONCHAIN_V1_EXT)
            .expect("onchain melt quote relative path");
        let msg = wire_melt::MeltQuoteOnchainRequest {
            inputs: crate::wire::attestation::AttestedFingerprints {
                inputs,
                attestation,
            },
            address: recipient,
            amount,
            network_fee,
            wallet_key,
        };
        let wire_melt::MeltQuoteOnchainResponse {
            content,
            commitment,
        } = self
            .cl
            .post(url, &msg)
            .await
            .map_err(treasury::Error::from)?;
        signature::schnorr_verify_b64(&content, &commitment, &mint_pk.x_only_public_key().0)?;
        Ok((content, commitment))
    }

    pub async fn onchain_mint_quote(
        &self,
        blinds: Vec<cashu::BlindedMessage>,
        wallet_key: cashu::PublicKey,
        mint_pk: secp::PublicKey,
    ) -> Result<wire_mint::OnchainMintQuoteResponse> {
        let url = self
            .base
            .join(treasury::web_ep::MINTQUOTE_ONCHAIN_V2_EXT)
            .expect("onchain mint quote relative path");
        let msg = wire_mint::OnchainMintQuoteRequest {
            blinded_messages: blinds,
            wallet_key,
        };
        let response: wire_mint::OnchainMintQuoteResponse = self
            .cl
            .post(url, &msg)
            .await
            .map_err(treasury::Error::from)?;
        signature::schnorr_verify_b64(
            &response.content,
            &response.commitment,
            &mint_pk.x_only_public_key().0,
        )?;
        Ok(response)
    }

    #[deprecated(note = "use `onchain_mint_quote` instead")]
    pub async fn onchain_mint_quote_v1(
        &self,
        blinds: Vec<cashu::BlindedMessage>,
        wallet_key: cashu::PublicKey,
        mint_pk: secp::PublicKey,
    ) -> Result<wire_mint::OnchainMintQuoteResponse> {
        let url = self
            .base
            .join(treasury::web_ep::MINTQUOTE_ONCHAIN_V1_EXT)
            .expect("onchain mint quote relative path");
        let msg = wire_mint::OnchainMintQuoteRequest {
            blinded_messages: blinds,
            wallet_key,
        };
        let response: wire_mint::OnchainMintQuoteResponse = self
            .cl
            .post(url, &msg)
            .await
            .map_err(treasury::Error::from)?;
        signature::schnorr_verify_b64(
            &response.content,
            &response.commitment,
            &mint_pk.x_only_public_key().0,
        )?;
        Ok(response)
    }

    pub async fn onchain_melt_estimate(
        &self,
        amount: bitcoin::Amount,
        recipient: Option<bitcoin::Address<bitcoin::address::NetworkUnchecked>>,
    ) -> Result<wire_melt::MeltOnchainEstimateResponse> {
        let url = self
            .base
            .join(treasury::web_ep::MELT_ONCHAIN_ESTIMATE_V1_EXT)
            .expect("onchain melt estimate relative path");
        let msg = wire_melt::MeltOnchainEstimateRequest {
            amount,
            address: recipient,
        };
        let response = self
            .cl
            .post(url, &msg)
            .await
            .map_err(treasury::Error::from)?;
        Ok(response)
    }

    pub async fn onchain_melt_config(&self) -> Result<wire_melt::MeltOnchainConfigResponse> {
        let url = self
            .base
            .join(treasury::web_ep::MELT_ONCHAIN_CONFIG_V1_EXT)
            .expect("onchain melt config relative path");
        let response = self.cl.get(url, &[]).await.map_err(treasury::Error::from)?;
        Ok(response)
    }

    pub async fn onchain_melt(
        &self,
        qid: Uuid,
        inputs: Vec<ecash::Proof>,
    ) -> Result<bitcoin::Txid> {
        let url = self
            .base
            .join(treasury::web_ep::MELT_ONCHAIN_V1_EXT)
            .expect("onchain melt relative path");
        let msg = wire_melt::MeltOnchainRequest { quote: qid, inputs };
        let response: wire_melt::MeltOnchainResponse = self
            .cl
            .post(url, &msg)
            .await
            .map_err(treasury::Error::from)?;
        Ok(response.txid)
    }

    pub async fn onchain_mint(
        &self,
        qid: Uuid,
        mint_id: secp::PublicKey,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let url = self
            .base
            .join(treasury::web_ep::MINT_ONCHAIN_V1_EXT)
            .expect("onchain mint relative path");
        let msg = wire_mint::OnchainMintRequest {
            quote: qid,
            alpha_id: mint_id,
        };
        let response: cashu::MintResponse = self
            .cl
            .post(url, &msg)
            .await
            .map_err(treasury::Error::from)?;
        Ok(response.signatures)
    }

    // -------------------------------------------------------------------------
    // Clowder service – public endpoints
    // -------------------------------------------------------------------------

    pub async fn get_offline(
        &self,
        alpha_id: &secp::PublicKey,
    ) -> Result<wire_clowder::OfflineResponse> {
        let response = clowder::common::get_offline(
            &self.cl,
            &self.base,
            web_ep::FOREIGN_OFFLINE_V1_EXT,
            alpha_id,
        )
        .await?;
        Ok(response)
    }

    pub async fn get_status(
        &self,
        pubkey: &secp::PublicKey,
    ) -> Result<wire_clowder::AlphaStateResponse> {
        let response = clowder::common::get_status(
            &self.cl,
            &self.base,
            web_ep::FOREIGN_STATUS_V1_EXT,
            pubkey,
        )
        .await?;
        Ok(response)
    }

    pub async fn get_substitute(
        &self,
        alpha_id: &secp::PublicKey,
    ) -> Result<wire_clowder::ConnectedMintResponse> {
        let response = clowder::common::get_substitute(
            &self.cl,
            &self.base,
            clowder::web_ep::FOREIGN_SUBSTITUTE_V1_EXT,
            alpha_id,
        )
        .await?;
        Ok(response)
    }

    pub async fn get_active_keysets(
        &self,
        alpha_id: &secp::PublicKey,
    ) -> Result<cashu::KeysResponse> {
        let response = clowder::common::get_active_keysets(
            &self.cl,
            &self.base,
            clowder::web_ep::FOREIGN_ACTIVE_KEYSETS_V1_EXT,
            alpha_id,
        )
        .await?;
        Ok(response)
    }

    pub async fn post_path(
        &self,
        origin_mint_url: reqwest::Url,
    ) -> Result<wire_clowder::ConnectedMintsResponse> {
        let response = clowder::common::post_path(
            &self.cl,
            &self.base,
            web_ep::FOREIGN_PATH_V1_EXT,
            origin_mint_url,
        )
        .await?;
        Ok(response)
    }

    pub async fn get_info(&self) -> Result<wire_clowder::ClowderNodeInfo> {
        let response =
            clowder::common::get_info(&self.cl, &self.base, web_ep::LOCAL_INFO_V1_EXT).await?;
        Ok(response)
    }

    pub async fn get_betas(&self) -> Result<wire_clowder::ConnectedMintsResponse> {
        let url = self
            .base
            .join(clowder::web_ep::LOCAL_BETAS_V1_EXT)
            .expect("betas relative path");
        let response: wire_clowder::ConnectedMintsResponse =
            self.cl.get(url, &[]).await.map_err(clowder::Error::from)?;
        Ok(response)
    }

    pub async fn get_coverage(&self) -> Result<wire_clowder::Coverage> {
        let url = self
            .base
            .join(clowder::web_ep::LOCAL_COVERAGE_V1_EXT)
            .expect("coverage relative path");
        let response: wire_clowder::Coverage =
            self.cl.get(url, &[]).await.map_err(clowder::Error::from)?;
        Ok(response)
    }

    pub async fn post_attest_issuance(
        &self,
        request: &wire_attestation::IssuanceAttestationRequest,
    ) -> Result<wire_attestation::IssuanceAttestation> {
        let response = clowder::common::post_attest_issuance(
            &self.cl,
            &self.base,
            clowder::web_ep::ATTEST_ISSUANCE_V1_EXT,
            request,
        )
        .await?;
        Ok(response)
    }

    // -------------------------------------------------------------------------
    // Clowder – protests, answered by a beta of the accused alpha
    // -------------------------------------------------------------------------

    pub async fn protest_mint(
        &self,
        request: &wire_mint::MintProtestRequest,
    ) -> Result<ProtestOutcome<Vec<cashu::BlindSignature>>> {
        self.protest::<_, wire_mint::MintProtestResponse, _>(web_ep::PROTEST_MINT_V1_EXT, request)
            .await
    }

    pub async fn protest_swap(
        &self,
        request: &wire_swap::SwapProtestRequest,
    ) -> Result<ProtestOutcome<Vec<ecash::BlindSignature>>> {
        self.protest::<_, wire_swap::SwapProtestResponse, _>(web_ep::PROTEST_SWAP_V1_EXT, request)
            .await
    }

    pub async fn protest_melt(
        &self,
        request: &wire_melt::MeltProtestRequest,
    ) -> Result<ProtestOutcome<bitcoin::Txid>> {
        self.protest::<_, wire_melt::MeltProtestResponse, _>(web_ep::PROTEST_MELT_V1_EXT, request)
            .await
    }

    async fn protest<Request, Response, T>(
        &self,
        ep: &str,
        request: &Request,
    ) -> Result<ProtestOutcome<T>>
    where
        Request: serde::Serialize,
        Response:
            serde::de::DeserializeOwned + TryInto<ProtestOutcome<T>, Error = ProtestOutcomeError>,
    {
        let url = self.base.join(ep).expect("protest relative path");
        let response: Response = self
            .cl
            .post(url, request)
            .await
            .map_err(clowder::Error::from)?;
        Ok(response.try_into()?)
    }

    pub async fn derive_ebill_payment_address(
        &self,
        alpha_id: secp::PublicKey,
        bill_id: BillId,
        block_id: u64,
        previous_block_hash: bitcoin::hashes::sha256::Hash,
    ) -> Result<wire_clowder::DeriveEbillPaymentAddressResponse> {
        let response = clowder::common::derive_ebill_payment_address(
            &self.cl,
            &self.base,
            web_ep::LOCAL_DERIVE_EBILL_PAYMENT_ADDRESS_V1_EXT,
            alpha_id,
            bill_id,
            block_id,
            previous_block_hash,
        )
        .await?;
        Ok(response)
    }
}

#[cfg(test)]
mod protest_contract {
    use super::*;
    use crate::wire::{
        clowder::ClowderRejection,
        test_utils::{protest_entry as entry, protest_parse as parse},
    };

    async fn answer(
        server: &mut mockito::Server,
        ep: &str,
        request: &str,
        status: usize,
        body: serde_json::Value,
    ) -> mockito::Mock {
        server
            .mock("POST", ep)
            .match_body(mockito::Matcher::Json(entry(request)))
            .with_status(status)
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .expect(1)
            .create_async()
            .await
    }

    #[tokio::test]
    async fn client_posts_each_protest_to_its_endpoint() {
        let mut server = mockito::Server::new_async().await;
        let mint = answer(
            &mut server,
            web_ep::PROTEST_MINT_V1_EXT,
            "/mint/request",
            200,
            entry("/mint/resolved"),
        )
        .await;
        let swap = answer(
            &mut server,
            web_ep::PROTEST_SWAP_V1_EXT,
            "/swap/request",
            200,
            entry("/swap/rabid"),
        )
        .await;
        let melt = answer(
            &mut server,
            web_ep::PROTEST_MELT_V1_EXT,
            "/melt/request",
            200,
            entry("/melt/offline"),
        )
        .await;
        let client = Client::new(reqwest::Url::parse(&server.url()).unwrap());

        let request: wire_mint::MintProtestRequest = parse("/mint/request");
        let outcome = client.protest_mint(&request).await.expect("mint protest");
        let expected: wire_mint::MintProtestResponse = parse("/mint/resolved");
        assert_eq!(
            outcome,
            ProtestOutcome::Resolved(expected.signatures.expect("fixture signatures"))
        );

        let request: wire_swap::SwapProtestRequest = parse("/swap/request");
        let outcome = client.protest_swap(&request).await.expect("swap protest");
        assert_eq!(outcome, ProtestOutcome::Rabid);

        let request: wire_melt::MeltProtestRequest = parse("/melt/request");
        let outcome = client.protest_melt(&request).await.expect("melt protest");
        assert_eq!(outcome, ProtestOutcome::Offline);

        mint.assert_async().await;
        swap.assert_async().await;
        melt.assert_async().await;
    }

    #[tokio::test]
    async fn client_refuses_a_resolved_answer_without_its_result() {
        let mut server = mockito::Server::new_async().await;
        let melt = answer(
            &mut server,
            web_ep::PROTEST_MELT_V1_EXT,
            "/melt/request",
            200,
            entry("/melt/resolved_without_txid"),
        )
        .await;
        let client = Client::new(reqwest::Url::parse(&server.url()).unwrap());

        let request: wire_melt::MeltProtestRequest = parse("/melt/request");
        let result = client.protest_melt(&request).await;
        assert!(
            matches!(
                result,
                Err(Error::ProtestOutcome(
                    ProtestOutcomeError::ResolvedWithoutResult
                ))
            ),
            "{result:?}"
        );
        melt.assert_async().await;
    }

    #[tokio::test]
    async fn client_surfaces_the_beta_rejection() {
        let mut server = mockito::Server::new_async().await;
        let status = entry("/rejection/http_status").as_u64().unwrap() as usize;
        let swap = answer(
            &mut server,
            web_ep::PROTEST_SWAP_V1_EXT,
            "/swap/request",
            status,
            entry("/rejection/body"),
        )
        .await;
        let client = Client::new(reqwest::Url::parse(&server.url()).unwrap());

        let request: wire_swap::SwapProtestRequest = parse("/swap/request");
        let result = client.protest_swap(&request).await;
        let Err(Error::InvalidRequest(BRError::Clowder(body))) = result else {
            panic!("expected a clowder rejection, got {result:?}");
        };
        let rejection: ClowderRejection = serde_json::from_value(body).unwrap();
        assert_eq!(rejection, parse::<ClowderRejection>("/rejection/body"));
        swap.assert_async().await;
    }
}

/// The wallet-facing wire contract.
///
/// Wallets in the field were built against the previous shape of these messages, where every
/// amount was a `bitcoin::Amount` or a `cashu::Amount` rather than a plain `u64`. No field on
/// this surface was added, removed or renamed, so the only thing to prove is that the retype
/// is invisible on the wire. Each test stands in for one of the two directions:
///
/// - a *new* mint answering an *old* wallet: our output is parsed into structs declared with the
///   old field types, so anything renamed, retyped or dropped shows up as a failure here;
/// - an *old* wallet or mint talking to new code: byte fixtures captured from the previous
///   definitions are parsed into the current types.
#[cfg(all(test, feature = "test-utils"))]
mod wallet_wire_contract {
    use super::*;
    use crate::core_tests;
    use serde::Deserialize;

    /// `bitcoin::Amount` is a `u64` newtype with a derived serde impl, so a wallet still
    /// compiled against it must read our plain `u64` without noticing.
    #[derive(Debug, Deserialize)]
    struct LegacyProof {
        amount: bitcoin::Amount,
        id: ecash::Id,
        secret: cashu::secret::Secret,
        #[serde(rename = "C")]
        c: cashu::PublicKey,
    }

    #[derive(Debug, Deserialize)]
    struct LegacyBlindedMessage {
        amount: bitcoin::Amount,
        id: ecash::Id,
        #[serde(rename = "B_")]
        blinded_secret: cashu::PublicKey,
    }

    #[derive(Debug, Deserialize)]
    struct LegacyBlindSignature {
        amount: bitcoin::Amount,
        id: ecash::Id,
        #[serde(rename = "C_")]
        c: cashu::PublicKey,
    }

    #[derive(Debug, Deserialize)]
    struct LegacyProofFingerprint {
        keyset_id: ecash::Id,
        amount: u64,
    }

    /// `/v1/local/coverage`, whose five amounts were two different `Amount` newtypes.
    #[derive(Debug, Deserialize)]
    struct LegacyCoverage {
        debit_circulating_supply: cashu::Amount,
        credit_circulating_supply: cashu::Amount,
        onchain_collateral: bitcoin::Amount,
        ebill_collateral: bitcoin::Amount,
        eiou_collateral: u64,
    }

    #[derive(Debug, Deserialize)]
    struct LegacyMeltOnchainEstimateResponse {
        tx_vsize: u64,
        melt_fee: bitcoin::Amount,
        melt_fee_ppk: u64,
    }

    /// The quote status a wallet polls through `lookup`. Not covered by the frozen-byte
    /// tests, since it is JSON only.
    #[derive(Debug, Deserialize)]
    #[serde(tag = "status")]
    enum LegacyStatusReply {
        Offered {
            keyset_id: cashu::Id,
            discounted: bitcoin::Amount,
            wallet_pubkey: cashu::PublicKey,
        },
        MintingEnabled {
            keyset_id: cashu::Id,
            discounted: bitcoin::Amount,
            minted_amount: cashu::Amount,
        },
        /// Terminal status, the one shape that gained a `unit` an old wallet never saw
        OfferExpired {
            #[serde(with = "time::serde::rfc3339")]
            tstamp: time::OffsetDateTime,
            discounted: bitcoin::Amount,
        },
    }

    fn sample_proof() -> ecash::Proof {
        let keyset = core_tests::generate_random_ecash_keyset().1;
        core_tests::generate_random_ecash_proofs(&keyset, &[cashu::Amount::from(8u64)])
            .remove(0)
            .into()
    }

    #[test]
    fn a_legacy_wallet_still_reads_the_ecash_primitives() {
        let proof = sample_proof();
        let json = serde_json::to_string(&proof).expect("serialize");
        let legacy: LegacyProof = serde_json::from_str(&json).expect("legacy wallet parses proof");
        assert_eq!(legacy.amount.to_sat(), proof.amount);
        assert_eq!(legacy.id, proof.keyset_id);
        assert_eq!(legacy.secret, proof.secret);
        assert_eq!(legacy.c, proof.c);

        let secret = cashu::secret::Secret::new(rand::random::<u64>().to_string());
        let (blinded_secret, _) =
            cashu::dhke::blind_message(secret.as_bytes(), None).expect("blind");
        let message = ecash::BlindedMessage {
            amount: 16,
            keyset_id: proof.keyset_id,
            blinded_secret,
            witness: None,
        };
        let json = serde_json::to_string(&message).expect("serialize");
        let legacy: LegacyBlindedMessage = serde_json::from_str(&json).expect("legacy parses");
        assert_eq!(legacy.amount.to_sat(), message.amount);
        assert_eq!(legacy.id, message.keyset_id);
        assert_eq!(legacy.blinded_secret, message.blinded_secret);

        let signature = ecash::BlindSignature {
            amount: 32,
            keyset_id: proof.keyset_id,
            c: proof.c,
            dleq: None,
        };
        let json = serde_json::to_string(&signature).expect("serialize");
        let legacy: LegacyBlindSignature = serde_json::from_str(&json).expect("legacy parses");
        assert_eq!(legacy.amount.to_sat(), signature.amount);
        assert_eq!(legacy.id, signature.keyset_id);
        assert_eq!(legacy.c, signature.c);

        let fp = wire_keys::ProofFingerprint::try_from(proof.clone()).expect("fingerprint");
        let json = serde_json::to_string(&fp).expect("serialize");
        let legacy: LegacyProofFingerprint = serde_json::from_str(&json).expect("legacy parses");
        assert_eq!(legacy.amount, proof.amount);
        assert_eq!(legacy.keyset_id, proof.keyset_id);
    }

    #[test]
    fn a_legacy_wallet_still_reads_coverage_and_melt_estimates() {
        let coverage = wire_clowder::Coverage {
            debit_circulating_supply: 1,
            credit_circulating_supply: 2,
            onchain_collateral: bitcoin::Amount::from_sat(3),
            ebill_collateral: bitcoin::Amount::from_sat(4),
            eiou_collateral: 5,
        };
        let json = serde_json::to_string(&coverage).expect("serialize");
        let legacy: LegacyCoverage = serde_json::from_str(&json).expect("legacy wallet parses");
        assert_eq!(legacy.debit_circulating_supply, cashu::Amount::from(1u64));
        assert_eq!(legacy.credit_circulating_supply, cashu::Amount::from(2u64));
        assert_eq!(legacy.onchain_collateral.to_sat(), 3);
        assert_eq!(legacy.ebill_collateral.to_sat(), 4);
        assert_eq!(legacy.eiou_collateral, 5);

        let estimate = wire_melt::MeltOnchainEstimateResponse {
            tx_vsize: 140,
            feerates: vec![],
            melt_fee: bitcoin::Amount::from_sat(250),
            melt_fee_ppk: 2,
        };
        let json = serde_json::to_string(&estimate).expect("serialize");
        let legacy: LegacyMeltOnchainEstimateResponse =
            serde_json::from_str(&json).expect("legacy wallet parses");
        assert_eq!(legacy.tx_vsize, 140);
        assert_eq!(legacy.melt_fee.to_sat(), 250);
        assert_eq!(legacy.melt_fee_ppk, 2);
    }

    #[test]
    fn a_legacy_wallet_still_reads_the_quote_status_it_polls() {
        let keyset_id = cashu::Id::from(sample_proof().keyset_id);
        let wallet_pubkey = crate::ecash::public_key_secp2cashu(
            &crate::core::generate_random_keypair().public_key(),
        );
        let reply = wire_quotes::StatusReply::MintingEnabled {
            keyset_id,
            discounted: 1200,
            wallet_pubkey,
            minted_amount: 1000,
        };
        let json = serde_json::to_string(&reply).expect("serialize");
        let legacy: LegacyStatusReply = serde_json::from_str(&json).expect("legacy wallet parses");
        let LegacyStatusReply::MintingEnabled {
            keyset_id: kid,
            discounted,
            minted_amount,
        } = legacy
        else {
            panic!("variant tag changed: {json}");
        };
        assert_eq!(kid, keyset_id);
        assert_eq!(discounted.to_sat(), 1200);
        assert_eq!(minted_amount, cashu::Amount::from(1000u64));

        let offered = wire_quotes::StatusReply::Offered {
            keyset_id,
            expiration_date: time::macros::datetime!(2026-08-03 12:00:00 UTC),
            discounted: 1200,
            wallet_pubkey,
        };
        let json = serde_json::to_string(&offered).expect("serialize");
        let legacy: LegacyStatusReply = serde_json::from_str(&json).expect("legacy wallet parses");
        let LegacyStatusReply::Offered {
            keyset_id: kid,
            discounted,
            wallet_pubkey: pk,
        } = legacy
        else {
            panic!("variant tag changed: {json}");
        };
        assert_eq!(kid, keyset_id);
        assert_eq!(discounted.to_sat(), 1200);
        assert_eq!(pk, wallet_pubkey);

        // a terminal status now carries a unit; an old wallet must ignore it and still
        // read the amount it already knew about
        let expired = wire_quotes::StatusReply::OfferExpired {
            tstamp: time::macros::datetime!(2026-08-03 12:00:00 UTC),
            discounted: 1200,
            unit: ecash::CurrencyUnit::new("eur").expect("unit"),
        };
        let json = serde_json::to_string(&expired).expect("serialize");
        let legacy: LegacyStatusReply = serde_json::from_str(&json).expect("legacy wallet parses");
        let LegacyStatusReply::OfferExpired { tstamp, discounted } = legacy else {
            panic!("variant tag changed: {json}");
        };
        assert_eq!(discounted.to_sat(), 1200);
        assert_eq!(tstamp.unix_timestamp(), 1_785_758_400);

        // and a status written before the change parses into the current type
        let legacy_json = serde_json::json!({
            "status": "Offered",
            "keyset_id": keyset_id.to_string(),
            "expiration_date": "2026-08-03T12:00:00Z",
            "discounted": 1200,
            "wallet_pubkey": wallet_pubkey,
        });
        let parsed: wire_quotes::StatusReply = serde_json::from_value(legacy_json).expect("parse");
        assert!(
            matches!(parsed, wire_quotes::StatusReply::Offered { discounted, .. } if discounted == 1200)
        );
    }

    /// The other direction: payloads written by code predating this change, parsed by the
    /// current types. The `unit`-less bodies are what a deployed peer sends today.
    #[test]
    fn current_types_parse_payloads_written_before_the_change() {
        let proof = sample_proof();
        let legacy_proof = serde_json::json!({
            "amount": 8,
            "id": proof.keyset_id.to_string(),
            "secret": proof.secret,
            "C": proof.c,
        });
        let parsed: ecash::Proof = serde_json::from_value(legacy_proof).expect("parse");
        assert_eq!(parsed.amount, 8);
        assert_eq!(parsed.keyset_id, proof.keyset_id);

        let legacy_coverage = serde_json::json!({
            "debit_circulating_supply": 1, "credit_circulating_supply": 2,
            "onchain_collateral": 3, "ebill_collateral": 4, "eiou_collateral": 5,
        });
        let parsed: wire_clowder::Coverage =
            serde_json::from_value(legacy_coverage).expect("parse");
        assert_eq!(parsed.debit_circulating_supply, 1);
        assert_eq!(parsed.eiou_collateral, 5);

        let legacy_estimate = serde_json::json!({
            "tx_vsize": 140, "feerates": [], "melt_fee": 250, "melt_fee_ppk": 2,
        });
        let parsed: wire_melt::MeltOnchainEstimateResponse =
            serde_json::from_value(legacy_estimate).expect("parse");
        assert_eq!(parsed.melt_fee.to_sat(), 250);
    }
}
