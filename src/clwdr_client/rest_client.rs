// ----- standard library imports
// ----- extra library imports
use bitcoin::hashes::sha256::Hash as Sha256;
// ----- project imports
use super::ClowderClientError;
use super::Url;
use super::error::Result;
use super::jsonrpc_client::JsonRpcClient;
use super::model::*;
use crate::cashu::{
    CheckStateResponse, Id, KeysResponse, KeysetResponse, MintUrl, Proof,
    PublicKey as CashuPublicKey,
};
use crate::core::BillId;
use crate::wire::clowder::{
    AlphaStateResponse, BitcoinAmountResponse, ClowderNodeInfo, ConnectedMintResponse,
    ConnectedMintsResponse, DeriveEbillPaymentAddressRequest, DeriveEbillPaymentAddressResponse,
    EbillAmountResponse, EiouAmountResponse, MintCirculatingSupplyResponse, MintCollateralResponse,
    OfflineResponse, OnchainAddressRequest, OnchainAddressResponse, PathRequest, PerceivedState,
    SubstituteExchangeRequest, SupplyResponse, VerifyEbillMintPaymentRequest,
    VerifyMintPaymentRequest, VerifyMintPaymentResponse,
};
use crate::wire::keys::ProofFingerprint;
// ----- end imports

pub struct ClowderRestClient {
    base_url: Url,
    client: JsonRpcClient,
}

impl ClowderRestClient {
    pub fn new(base_url: Url) -> Self {
        Self {
            base_url,
            client: JsonRpcClient::new(),
        }
    }

    pub fn get_base_url(&self) -> Url {
        self.base_url.clone()
    }

    fn url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path)
            .map_err(|err| ClowderClientError::UrlParse(err.to_string()))
    }

    /// Determines the Beta nodes that this mint streams data to
    pub const LOCAL_BETAS_PATH: &'static str = "/local/betas";
    pub async fn get_betas(&self) -> Result<ConnectedMintsResponse> {
        let url = self.url(Self::LOCAL_BETAS_PATH)?;
        self.client.get(url).await
    }

    /// Determines the Alpha nodes that stream data toward this mint
    pub const LOCAL_ALPHAS_PATH: &'static str = "/local/alphas";
    pub async fn get_alphas(&self) -> Result<ConnectedMintsResponse> {
        let url = self.url(Self::LOCAL_ALPHAS_PATH)?;
        self.client.get(url).await
    }

    /// Get the mint url for a node id
    pub const FOREIGN_URL_PATH: &'static str = "/foreign/url/{node_id}";
    pub async fn get_mint_url(
        &self,
        node_id: bitcoin::secp256k1::PublicKey,
    ) -> Result<MintUrlResponse> {
        let path = Self::FOREIGN_URL_PATH.replace("{node_id}", &node_id.to_string());
        let url = self.url(&path)?;
        self.client.get(url).await
    }

    /// Sign proofs HTLC Witness with the node's private key
    pub const LOCAL_SIGN_PROOFS_PATH: &'static str = "/local/sign_proofs";
    pub async fn post_sign_proofs(&self, proofs: &[Proof]) -> Result<ProofsResponse> {
        let url = self.url(Self::LOCAL_SIGN_PROOFS_PATH)?;
        self.client
            .post(
                url,
                &ProofsRequest {
                    proofs: proofs.to_vec(),
                },
            )
            .await
    }

    /// Validate HTLC lock on wallet proofs
    pub const LOCAL_VALIDATE_WALLET_LOCK_PATH: &'static str = "/local/validate/wallet_lock";
    pub async fn post_validate_wallet_lock(&self, proofs: &[Proof]) -> Result<SuccessResponse> {
        let url = self.url(Self::LOCAL_VALIDATE_WALLET_LOCK_PATH)?;
        self.client
            .post(
                url,
                &ProofsRequest {
                    proofs: proofs.to_vec(),
                },
            )
            .await
    }

    /// Validate HTLC lock on intermint treasury proofs
    pub const LOCAL_VALIDATE_ALPHA_LOCK_PATH: &'static str = "/local/validate/alpha_lock";
    pub async fn post_validate_alpha_lock(&self, proofs: &[Proof]) -> Result<SuccessResponse> {
        let url = self.url(Self::LOCAL_VALIDATE_ALPHA_LOCK_PATH)?;
        self.client
            .post(
                url,
                &ProofsRequest {
                    proofs: proofs.to_vec(),
                },
            )
            .await
    }

    /// Determines the shortest path to origin of the eCash
    pub const FOREIGN_PATH_PATH: &'static str = "/foreign/path";
    pub async fn post_path(&self, origin_mint_url: MintUrl) -> Result<ConnectedMintsResponse> {
        let url = self.url(Self::FOREIGN_PATH_PATH)?;
        let origin_mint_url = Url::parse(&origin_mint_url.to_string())
            .map_err(|e| ClowderClientError::UrlParse(e.to_string()))?;
        self.client
            .post(url, &PathRequest { origin_mint_url })
            .await
    }

    /// Determines whether proofs are spent or not
    pub const FOREIGN_CHECKSTATE_PATH: &'static str = "/foreign/checkstate/{pubkey}";
    pub async fn post_checkstate(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
        keyset_ids: Vec<Id>,
        proof_ys: Vec<CashuPublicKey>,
    ) -> Result<CheckStateResponse> {
        let req = CheckStateRequest {
            ys: proof_ys,
            ids: keyset_ids,
        };

        let path = Self::FOREIGN_CHECKSTATE_PATH.replace("{pubkey}", &pubkey.to_string());
        let url = self.url(&path)?;
        self.client.post(url, &req).await
    }

    /// Obtains the mint keyset
    pub const FOREIGN_KEYS_PATH: &'static str = "/foreign/{alpha_id}/keys/{keyset_id}";
    pub async fn get_keyset(
        &self,
        alpha_id: &bitcoin::secp256k1::PublicKey,
        keyset_id: &Id,
    ) -> Result<KeysResponse> {
        let path = Self::FOREIGN_KEYS_PATH
            .replace("{alpha_id}", &alpha_id.to_string())
            .replace("{keyset_id}", &keyset_id.to_string());
        let url = self.url(&path)?;
        self.client.get(url).await
    }

    pub const LOCAL_COMMITMENT_SUBSTITUTE_PATH: &'static str = "/local/commitment/substitute";
    pub async fn post_commitment_substitute(
        &self,
        proofs: Vec<ProofFingerprint>,
        locks: Vec<Sha256>,
        wallet_pubkey: bitcoin::secp256k1::PublicKey,
    ) -> Result<bitcoin::secp256k1::schnorr::Signature> {
        let payload = SubstituteExchangeRequest {
            proofs,
            locks,
            wallet_pubkey,
        };

        let url = self.url(Self::LOCAL_COMMITMENT_SUBSTITUTE_PATH)?;
        self.client.post(url, &payload).await
    }

    /// Obtains the mint keyset info (status)
    pub const FOREIGN_KEYSET_PATH: &'static str = "/foreign/{alpha_id}/keyset/{keyset_id}";
    pub async fn get_keyset_info(
        &self,
        alpha_id: &bitcoin::secp256k1::PublicKey,
        keyset_id: &Id,
    ) -> Result<KeysetResponse> {
        let path = Self::FOREIGN_KEYSET_PATH
            .replace("{alpha_id}", &alpha_id.to_string())
            .replace("{keyset_id}", &keyset_id.to_string());
        let url = self.url(&path)?;
        self.client.get(url).await
    }

    /// Obtains all active keysets for a mint
    pub const FOREIGN_ACTIVE_KEYSETS_PATH: &'static str = "/foreign/{alpha_id}/active_keysets";
    pub async fn get_active_keysets(
        &self,
        alpha_id: &bitcoin::secp256k1::PublicKey,
    ) -> Result<KeysResponse> {
        let path = Self::FOREIGN_ACTIVE_KEYSETS_PATH.replace("{alpha_id}", &alpha_id.to_string());
        let url = self.url(&path)?;
        self.client.get(url).await
    }

    /// Returns the minturl of the substitute Beta mint when Alpha is offline
    pub const LOCAL_SUBSTITUTE_PATH: &'static str = "/local/substitute";
    pub async fn post_determine_substitute_address(
        &self,
        mint_url: MintUrl,
    ) -> Result<MintUrlResponse> {
        let url = self.url(Self::LOCAL_SUBSTITUTE_PATH)?;
        self.client.post(url, &MintUrlRequest { mint_url }).await
    }

    /// Returns the state of the mint as seen by its beta validators
    pub const LOCAL_PERCEIVED_STATE_PATH: &'static str = "/local/perceived_state";
    pub async fn get_mint_perceived_state(&self) -> Result<PerceivedState> {
        let url = self.url(Self::LOCAL_PERCEIVED_STATE_PATH)?;
        self.client.get(url).await
    }

    pub const FOREIGN_SUBSTITUTE_PATH: &'static str = "/foreign/substitute/{alpha_id}";
    pub async fn get_substitute(
        &self,
        alpha_id: bitcoin::secp256k1::PublicKey,
    ) -> Result<ConnectedMintResponse> {
        let path = Self::FOREIGN_SUBSTITUTE_PATH.replace("{alpha_id}", &alpha_id.to_string());
        let url = self.url(&path)?;
        self.client.get(url).await
    }

    pub const FOREIGN_STATUS_PATH: &'static str = "/foreign/status/{pubkey}";
    pub async fn get_status(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
    ) -> Result<AlphaStateResponse> {
        let path = Self::FOREIGN_STATUS_PATH.replace("{pubkey}", &pubkey.to_string());
        let url = self.url(&path)?;
        self.client.get(url).await
    }

    /// Determines whether HTLC-locked intermint proofs are validly issued in Clowder
    /// Fails on non-HTLC-proofs
    pub const FOREIGN_VERIFY_PROOFS_PATH: &'static str = "/foreign/verify_proofs/{pubkey}";
    pub async fn post_verify_proofs(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
        proofs: Vec<Proof>,
    ) -> Result<IntermintValidProofs> {
        let path = Self::FOREIGN_VERIFY_PROOFS_PATH.replace("{pubkey}", &pubkey.to_string());
        let url = self.url(&path)?;
        self.client.post(url, &ProofsRequest { proofs }).await
    }

    /// Determines whether the fingerprint proofs are valid of an offline intermint exchange, requires DLEQ
    pub const FOREIGN_VERIFY_FINGERPRINTS_PATH: &'static str =
        "/foreign/verify_fingerprints/{pubkey}";
    pub async fn post_verify_fingerprints(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
        proofs: Vec<ProofFingerprint>,
    ) -> Result<ValidFingerprints> {
        let path = Self::FOREIGN_VERIFY_FINGERPRINTS_PATH.replace("{pubkey}", &pubkey.to_string());
        let url = self.url(&path)?;
        self.client.post(url, &FingerprintRequest { proofs }).await
    }

    pub const FOREIGN_OFFLINE_PATH: &'static str = "/foreign/offline/{pubkey}";
    pub async fn get_offline(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
    ) -> Result<OfflineResponse> {
        let path = Self::FOREIGN_OFFLINE_PATH.replace("{pubkey}", &pubkey.to_string());
        let url = self.url(&path)?;
        self.client.get(url).await
    }

    /// Get the last offline timestamp for an alpha node as perceived by this beta
    pub const FOREIGN_LAST_OFFLINE_PATH: &'static str = "/foreign/last_offline/{pubkey}";
    pub async fn get_last_offline(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
    ) -> Result<LastOfflineResponse> {
        let path = Self::FOREIGN_LAST_OFFLINE_PATH.replace("{pubkey}", &pubkey.to_string());
        let url = self.url(&path)?;
        self.client.get(url).await
    }

    /// Determines the origin of the Proofs, one of the Alpha mints that stream to this mint
    pub const FOREIGN_PROOFS_ORIGIN_PATH: &'static str = "/foreign/proofs_origin";
    pub async fn post_proofs_origin(&self, proofs: Vec<Proof>) -> Result<IntermintOriginResponse> {
        let url = self.url(Self::FOREIGN_PROOFS_ORIGIN_PATH)?;
        self.client.post(url, &ProofsRequest { proofs }).await
    }

    /// Determines the origin of the Proofs, one of the Alpha mints that stream to this mint
    pub const FOREIGN_FINGERPRINTS_ORIGIN_PATH: &'static str = "/foreign/fingerprints_origin";
    pub async fn post_fingerprints_origin(
        &self,
        proofs: Vec<ProofFingerprint>,
    ) -> Result<IntermintOriginResponse> {
        let url = self.url(Self::FOREIGN_FINGERPRINTS_ORIGIN_PATH)?;
        self.client.post(url, &FingerprintRequest { proofs }).await
    }

    /// Request a new mint address using a quote_id as tweak
    pub const LOCAL_REQUEST_ADDRESS_PATH: &'static str = "/local/request_address";
    pub async fn request_mint_address(
        &self,
        quote_id: uuid::Uuid,
        keyset_id: Id,
    ) -> Result<OnchainAddressResponse> {
        let url = self.url(Self::LOCAL_REQUEST_ADDRESS_PATH)?;
        let req = OnchainAddressRequest {
            keyset_id,
            quote_id,
        };
        self.client.post(url, &req).await
    }

    /// Verify if payment has been received for a mint quote
    pub const LOCAL_VERIFY_PAYMENT_PATH: &'static str = "/local/verify_payment";
    pub async fn verify_mint_payment(
        &self,
        quote_id: uuid::Uuid,
        keyset_id: Id,
        min_confirmations: u32,
    ) -> Result<VerifyMintPaymentResponse> {
        let url = self.url(Self::LOCAL_VERIFY_PAYMENT_PATH)?;
        let req = VerifyMintPaymentRequest {
            quote_id,
            keyset_id,
            min_confirmations,
        };
        self.client.post(url, &req).await
    }

    /// Verify if payment has been received for an E-Bill mint quote
    pub const LOCAL_VERIFY_EBILL_PAYMENT_PATH: &'static str = "/local/verify_ebill_payment";
    pub async fn verify_ebill_payment(&self, bill_id: BillId) -> Result<VerifyMintPaymentResponse> {
        let url = self.url(Self::LOCAL_VERIFY_EBILL_PAYMENT_PATH)?;
        let req = VerifyEbillMintPaymentRequest { bill_id };
        self.client.post(url, &req).await
    }

    /// Return the payment address for the given bill metadata for verification
    pub const LOCAL_DERIVE_EBILL_PAYMENT_ADDRESS: &'static str =
        "/local/derive_ebill_payment_address";
    pub async fn derive_ebill_payment_address(
        &self,
        bill_id: BillId,
        block_id: u64,
        previous_block_hash: bitcoin::hashes::sha256::Hash,
    ) -> Result<DeriveEbillPaymentAddressResponse> {
        let url = self.url(Self::LOCAL_DERIVE_EBILL_PAYMENT_ADDRESS)?;
        let req = DeriveEbillPaymentAddressRequest {
            bill_id,
            block_id,
            previous_block_hash,
        };
        self.client.post(url, &req).await
    }

    pub const FOREIGN_COLLATERAL_ONCHAIN_PATH: &'static str =
        "/foreign/collateral_onchain/{pubkey}";
    pub async fn get_collateral_onchain(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
    ) -> Result<BitcoinAmountResponse> {
        let path = Self::FOREIGN_COLLATERAL_ONCHAIN_PATH.replace("{pubkey}", &pubkey.to_string());
        let url = self.url(&path)?;
        self.client.get(url).await
    }

    pub const FOREIGN_COLLATERAL_EBILL_PATH: &'static str = "/foreign/collateral_ebill/{pubkey}";
    pub async fn get_collateral_ebill(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
    ) -> Result<EbillAmountResponse> {
        let path = Self::FOREIGN_COLLATERAL_EBILL_PATH.replace("{pubkey}", &pubkey.to_string());
        let url = self.url(&path)?;
        self.client.get(url).await
    }

    pub const FOREIGN_COLLATERAL_EIOU_PATH: &'static str = "/foreign/collateral_eiou/{pubkey}";
    pub async fn get_collateral_eiou(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
    ) -> Result<EiouAmountResponse> {
        let path = Self::FOREIGN_COLLATERAL_EIOU_PATH.replace("{pubkey}", &pubkey.to_string());
        let url = self.url(&path)?;
        self.client.get(url).await
    }

    pub const FOREIGN_CIRCULATING_SUPPLY_PATH: &'static str =
        "/foreign/circulating_supply/{pubkey}";
    pub async fn get_circulating_supply(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
    ) -> Result<SupplyResponse> {
        let path = Self::FOREIGN_CIRCULATING_SUPPLY_PATH.replace("{pubkey}", &pubkey.to_string());
        let url = self.url(&path)?;
        self.client.get(url).await
    }

    pub const FOREIGN_KEYSET_MINTS_PATH: &'static str =
        "/foreign/keyset_mints/{pubkey}/{keyset_id}";
    pub async fn get_keyset_mints(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
        keyset_id: &Id,
    ) -> Result<AmountResponse> {
        let path = Self::FOREIGN_KEYSET_MINTS_PATH
            .replace("{pubkey}", &pubkey.to_string())
            .replace("{keyset_id}", &keyset_id.to_string());
        let url = self.url(&path)?;
        self.client.get(url).await
    }

    pub const FOREIGN_KEYSET_BURNS_PATH: &'static str =
        "/foreign/keyset_burns/{pubkey}/{keyset_id}";
    pub async fn get_keyset_burns(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
        keyset_id: &Id,
    ) -> Result<AmountResponse> {
        let path = Self::FOREIGN_KEYSET_BURNS_PATH
            .replace("{pubkey}", &pubkey.to_string())
            .replace("{keyset_id}", &keyset_id.to_string());
        let url = self.url(&path)?;
        self.client.get(url).await
    }

    pub const LOCAL_COLLATERAL_PATH: &'static str = "/local/collateral";
    pub async fn get_mint_collateral(&self) -> Result<MintCollateralResponse> {
        let url = self.url(Self::LOCAL_COLLATERAL_PATH)?;
        self.client.get(url).await
    }

    pub const LOCAL_CIRCULATING_SUPPLY_PATH: &'static str = "/local/circulating_supply";
    pub async fn get_mint_circulating_supply(&self) -> Result<MintCirculatingSupplyResponse> {
        let url = self.url(Self::LOCAL_CIRCULATING_SUPPLY_PATH)?;
        self.client.get(url).await
    }

    pub const FOREIGN_MINT_ONCHAIN_SIGNATURES_PATH: &'static str =
        "/foreign/mint_signatures/{alpha_id}/{quote_id}";
    pub async fn fetch_mint_onchain_signatures(
        &self,
        alpha_id: &bitcoin::secp256k1::PublicKey,
        quote_id: &uuid::Uuid,
    ) -> Result<Option<Vec<crate::cashu::nuts::BlindSignature>>> {
        let path = Self::FOREIGN_MINT_ONCHAIN_SIGNATURES_PATH
            .replace("{alpha_id}", &alpha_id.to_string())
            .replace("{quote_id}", &quote_id.to_string());
        let url = self.url(&path)?;
        self.client.get(url).await
    }

    pub const FOREIGN_MINT_ONCHAIN_PATH: &'static str = "/foreign/mint/onchain";
    pub async fn fetch_mint_onchain(
        &self,
        request: &crate::wire::mint::OnchainMintRequest,
    ) -> Result<crate::wire::mint::MintResponse> {
        let url = self.url(Self::FOREIGN_MINT_ONCHAIN_PATH)?;
        self.client.post(url, request).await
    }

    pub const FOREIGN_PROTEST_MINT_PATH: &'static str = "/foreign/protest_mint";
    pub async fn protest_mint(
        &self,
        request: crate::wire::mint::MintProtestRequest,
    ) -> Result<crate::wire::mint::MintProtestResponse> {
        let url = self.url(Self::FOREIGN_PROTEST_MINT_PATH)?;
        self.client.post(url, &request).await
    }

    pub const FOREIGN_PROTEST_SWAP_PATH: &'static str = "/foreign/protest_swap";
    pub async fn protest_swap(
        &self,
        request: crate::wire::swap::SwapProtestRequest,
    ) -> Result<crate::wire::swap::SwapProtestResponse> {
        let url = self.url(Self::FOREIGN_PROTEST_SWAP_PATH)?;
        self.client.post(url, &request).await
    }

    pub const FOREIGN_PROTEST_MELT_PATH: &'static str = "/foreign/protest_melt";
    pub async fn protest_melt(
        &self,
        request: crate::wire::melt::MeltProtestRequest,
    ) -> Result<crate::wire::melt::MeltProtestResponse> {
        let url = self.url(Self::FOREIGN_PROTEST_MELT_PATH)?;
        self.client.post(url, &request).await
    }

    pub const LOCAL_INFO_PATH: &'static str = "/local/info";
    pub async fn get_info(&self) -> Result<ClowderNodeInfo> {
        let url = self.url(Self::LOCAL_INFO_PATH)?;
        self.client.get(url).await
    }
}
