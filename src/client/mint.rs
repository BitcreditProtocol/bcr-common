// ----- standard library imports
// ----- extra library imports
use bitcoin::secp256k1;
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
    wire::{
        attestation as wire_attestation, clowder as wire_clowder, exchange as wire_exchange,
        keys as wire_keys, melt as wire_melt, mint as wire_mint, quotes as wire_quotes,
        swap as wire_swap,
    },
};

// ----- end imports

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

    #[error("cdk::nut20 {0}")]
    Cdk20(#[from] cashu::nut20::Error),
    #[error("borsh sign error {0}")]
    BorshSign(#[from] BorshMsgSignatureError),
}

impl std::convert::From<jsonrpc::Error> for Error {
    fn from(value: jsonrpc::Error) -> Self {
        match value {
            jsonrpc::Error::ResourceNotFound(e) => Error::ResourceNotFound(e),
            jsonrpc::Error::InvalidRequest(e) => Error::InvalidRequest(e),
            jsonrpc::Error::Internal(e) => Error::Internal(e),
            jsonrpc::Error::Reqwest(e) => Error::Reqwest(e),
        }
    }
}

impl std::convert::From<core::Error> for Error {
    fn from(value: core::Error) -> Self {
        match value {
            core::Error::ResourceNotFound(e) => Error::ResourceNotFound(e),
            core::Error::InvalidRequest(e) => Error::InvalidRequest(e),
            core::Error::Internal(e) => Error::Internal(e),
            core::Error::Reqwest(e) => Error::Reqwest(e),
            core::Error::NUT20(e) => Error::Cdk20(e),
            core::Error::BorshSign(e) => Error::BorshSign(e),
        }
    }
}

impl std::convert::From<treasury::Error> for Error {
    fn from(value: treasury::Error) -> Self {
        match value {
            treasury::Error::ResourceNotFound(e) => Error::ResourceNotFound(e),
            treasury::Error::InvalidRequest(e) => Error::InvalidRequest(e),
            treasury::Error::Internal(e) => Error::Internal(e),
            treasury::Error::Reqwest(e) => Error::Reqwest(e),
            treasury::Error::NUT20(e) => Error::Cdk20(e),
        }
    }
}

impl std::convert::From<clowder::Error> for Error {
    fn from(value: clowder::Error) -> Self {
        match value {
            clowder::Error::ResourceNotFound(e) => Error::ResourceNotFound(e),
            clowder::Error::InvalidRequest(e) => Error::InvalidRequest(e),
            clowder::Error::Internal(e) => Error::Internal(e),
            clowder::Error::Reqwest(e) => Error::Reqwest(e),
        }
    }
}

const CACHED_EPS: [(&str, reqwest::Method); 6] = [
    (core::web_ep::SWAP_COMMIT_V1_EXT, reqwest::Method::POST),
    (core::web_ep::SWAP_V1_EXT, reqwest::Method::POST),
    (core::web_ep::SIGNED_SWAP_V1_EXT, reqwest::Method::POST),
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

    pub async fn keys(&self, kid: cashu::Id) -> Result<cashu::KeySet> {
        let result =
            core::common::keys(&self.cl, &self.base, core::web_ep::KEYS_V1_EXT, kid).await?;
        Ok(result)
    }

    pub async fn keyset_info(&self, kid: cashu::Id) -> Result<cashu::KeySetInfo> {
        let result =
            core::common::keyset_info(&self.cl, &self.base, core::web_ep::KEYSET_INFO_V1_EXT, kid)
                .await?;
        Ok(result)
    }

    pub async fn list_keyset_info(
        &self,
        filters: wire_keys::KeysetInfoFilters,
    ) -> Result<Vec<cashu::KeySetInfo>> {
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
        outputs: Vec<cashu::BlindedMessage>,
        expiry: u64,
        wallet_pk: bitcoin::secp256k1::PublicKey,
        mint_pk: bitcoin::secp256k1::PublicKey,
        attestation: crate::wire::attestation::IssuanceAttestation,
    ) -> Result<(String, bitcoin::secp256k1::schnorr::Signature)> {
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

    // content is serialized wire_swap::SignedSwapRequestContent
    pub async fn signed_swap(
        &self,
        content: String,
        signature: bitcoin::secp256k1::schnorr::Signature,
        mint_id: bitcoin::secp256k1::PublicKey,
        commitment: bitcoin::secp256k1::schnorr::Signature,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let url = self
            .base
            .join(core::web_ep::SIGNED_SWAP_V1_EXT)
            .expect("signed swap relative path");
        let msg = wire_swap::SignedSwapRequest {
            content,
            signature,
            mint_id,
            commitment,
        };
        let response: wire_swap::SwapResponse = self.cl.post(url, &msg).await?;
        Ok(response.signatures)
    }

    pub async fn swap(
        &self,
        inputs: Vec<cashu::Proof>,
        outputs: Vec<cashu::BlindedMessage>,
        commitment: bitcoin::secp256k1::schnorr::Signature,
    ) -> Result<Vec<cashu::BlindSignature>> {
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
        let response: cashu::RestoreResponse = self.cl.post(url, &msg).await?;
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
            .join(quote::web_ep::ENQUIRE_V1_EXT)
            .expect("enquire relative path");
        let response: wire_quotes::EnquireReply = self.cl.post(url, &signed).await?;
        Ok(response.id)
    }

    pub async fn lookup(&self, qid: Uuid) -> Result<wire_quotes::StatusReply> {
        assert!(quote::web_ep::LOOKUP_V1_EXT.contains("{qid}"));
        let url = self
            .base
            .join(&quote::web_ep::LOOKUP_V1_EXT.replace("{qid}", &qid.to_string()))
            .expect("lookup relative path");
        let response: wire_quotes::StatusReply = self.cl.get(url, &[]).await?;
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
            .await?;
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
            .await?;
        Ok(())
    }

    pub async fn cancel_enquiry(&self, qid: Uuid) -> Result<()> {
        assert!(quote::web_ep::RESOLVE_V1_EXT.contains("{qid}"));
        let url = self
            .base
            .join(&quote::web_ep::RESOLVE_V1_EXT.replace("{qid}", &qid.to_string()))
            .expect("cancel enquiry relative path");
        self.cl.delete(url, &[]).await?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Treasury service – public endpoints
    // -------------------------------------------------------------------------

    pub async fn exchange_online(
        &self,
        proofs: Vec<cashu::Proof>,
        exchange_path: Vec<secp256k1::PublicKey>,
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
        mint_pk: secp256k1::PublicKey,
    ) -> Result<(Vec<cashu::Proof>, secp256k1::schnorr::Signature)> {
        let response = treasury::common::exchange_offline_raw(
            &self.cl,
            &self.base,
            treasury::web_ep::EXCHANGE_OFFLINE_V1_EXT,
            fingerprints,
            hashes,
            wallet_pk,
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
        let response: cashu::MintResponse = self.cl.post(url, &msg).await?;
        Ok(response.signatures)
    }

    /// target: the amount you expect to receive in recipient
    /// return: serialized (wire_melt::MeltQuoteOnchainResponseBody, signature)
    pub async fn onchain_melt_quote(
        &self,
        inputs: Vec<wire_keys::ProofFingerprint>,
        recipient: bitcoin::Address<bitcoin::address::NetworkUnchecked>,
        wallet_key: cashu::PublicKey,
        mint_pk: secp256k1::PublicKey,
        attestation: crate::wire::attestation::IssuanceAttestation,
    ) -> Result<(String, secp256k1::schnorr::Signature)> {
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
            wallet_key,
        };
        let wire_melt::MeltQuoteOnchainResponse {
            content,
            commitment,
        } = self.cl.post(url, &msg).await?;
        signature::schnorr_verify_b64(&content, &commitment, &mint_pk.x_only_public_key().0)?;
        Ok((content, commitment))
    }

    pub async fn onchain_mint_quote(
        &self,
        blinds: Vec<cashu::BlindedMessage>,
        wallet_key: cashu::PublicKey,
        mint_pk: secp256k1::PublicKey,
    ) -> Result<wire_mint::OnchainMintQuoteResponse> {
        let url = self
            .base
            .join(treasury::web_ep::MINTQUOTE_ONCHAIN_V1_EXT)
            .expect("onchain mint quote relative path");
        let msg = wire_mint::OnchainMintQuoteRequest {
            blinded_messages: blinds,
            wallet_key,
        };
        let response: wire_mint::OnchainMintQuoteResponse = self.cl.post(url, &msg).await?;
        signature::schnorr_verify_b64(
            &response.content,
            &response.commitment,
            &mint_pk.x_only_public_key().0,
        )?;
        Ok(response)
    }

    pub async fn onchain_melt(
        &self,
        qid: Uuid,
        inputs: Vec<cashu::Proof>,
    ) -> Result<bitcoin::Txid> {
        let url = self
            .base
            .join(treasury::web_ep::MELT_ONCHAIN_V1_EXT)
            .expect("onchain melt relative path");
        let msg = wire_melt::MeltOnchainRequest { quote: qid, inputs };
        let response: wire_melt::MeltOnchainResponse = self.cl.post(url, &msg).await?;
        Ok(response.txid)
    }

    pub async fn onchain_mint(
        &self,
        qid: Uuid,
        mint_id: secp256k1::PublicKey,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let url = self
            .base
            .join(treasury::web_ep::MINT_ONCHAIN_V1_EXT)
            .expect("onchain mint relative path");
        let msg = wire_mint::OnchainMintRequest {
            quote: qid,
            alpha_id: mint_id,
        };
        let response: cashu::MintResponse = self.cl.post(url, &msg).await?;
        Ok(response.signatures)
    }

    // -------------------------------------------------------------------------
    // Clowder service – public endpoints
    // -------------------------------------------------------------------------

    pub async fn get_offline(
        &self,
        alpha_id: &secp256k1::PublicKey,
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
        pubkey: &bitcoin::secp256k1::PublicKey,
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
        alpha_id: &secp256k1::PublicKey,
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
        alpha_id: &secp256k1::PublicKey,
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
        let response: wire_clowder::ConnectedMintsResponse = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn get_coverage(&self) -> Result<wire_clowder::Coverage> {
        let url = self
            .base
            .join(clowder::web_ep::LOCAL_COVERAGE_V1_EXT)
            .expect("coverage relative path");
        let response: wire_clowder::Coverage = self.cl.get(url, &[]).await?;
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

    pub async fn derive_ebill_payment_address(
        &self,
        alpha_id: secp256k1::PublicKey,
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
