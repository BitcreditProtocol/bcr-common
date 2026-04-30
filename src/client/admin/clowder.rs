// ----- standard library imports
// ----- extra library imports
use bitcoin::hashes::sha256::Hash as Sha256;
use thiserror::Error;
// ----- local imports
use crate::{
    cashu::{Id, KeysResponse, KeysetResponse, Proof},
    client::admin::jsonrpc,
    core::BillId,
    wire::{
        clowder::{self as wire_clowder, messages as clwdr_msgs},
        exchange as wire_exchange, keys as wire_keys,
    },
};

// ----- end imports

pub mod admin_ep {
    pub const FOREIGN_CHECKSTATE_V1: &str = "/foreign/checkstate/{pubkey}";
    pub const FOREIGN_CIRCULATING_SUPPLY_V1: &str = "/foreign/circulating_supply/{pubkey}";
    pub const FOREIGN_COLLATERAL_EBILL_V1: &str = "/foreign/collateral_ebill/{pubkey}";
    pub const FOREIGN_COLLATERAL_EIOU_V1: &str = "/foreign/collateral_eiou/{pubkey}";
    pub const FOREIGN_COLLATERAL_ONCHAIN_V1: &str = "/foreign/collateral_onchain/{pubkey}";
    pub const FOREIGN_FINGERPRINTS_ORIGIN_V1: &str = "/foreign/fingerprints_origin";
    pub const FOREIGN_KEYSET_BURNS_V1: &str = "/foreign/keyset_burns/{pubkey}/{keyset_id}";
    pub const FOREIGN_KEYSET_MINTS_V1: &str = "/foreign/keyset_mints/{pubkey}/{keyset_id}";
    pub const FOREIGN_KEYSET_V1: &str = "/foreign/{pubkey}/keyset/{keyset_id}";
    pub const FOREIGN_KEYS_V1: &str = "/foreign/{pubkey}/keys/{keyset_id}";
    pub const FOREIGN_LAST_OFFLINE_V1: &str = "/foreign/last_offline/{pubkey}";
    pub const FOREIGN_MINT_ONCHAIN_SIGNATURES_V1: &str =
        "/foreign/mint_signatures/{pubkey}/{quote_id}";
    pub const FOREIGN_MINT_ONCHAIN_V1: &str = "/foreign/mint/onchain";
    pub const FOREIGN_PROOFS_ORIGIN_V1: &str = "/foreign/proofs_origin";
    pub const FOREIGN_PROTEST_MELT_V1: &str = "/foreign/protest_melt";
    pub const FOREIGN_PROTEST_MINT_V1: &str = "/foreign/protest_mint";
    pub const FOREIGN_PROTEST_SWAP_V1: &str = "/foreign/protest_swap";
    pub const FOREIGN_URL_V1: &str = "/foreign/url/{pubkey}";
    pub const FOREIGN_VERIFY_FINGERPRINTS_V1: &str = "/foreign/verify_fingerprints/{pubkey}";
    pub const FOREIGN_VERIFY_PROOFS_V1: &str = "/foreign/verify_proofs/{pubkey}";
    pub const LOCAL_ALPHAS_V1: &str = "/local/alphas";
    pub const LOCAL_CIRCULATING_SUPPLY_V1: &str = "/local/circulating_supply";
    pub const LOCAL_COLLATERAL_V1: &str = "/local/collateral";
    pub const LOCAL_COMMITMENT_SUBSTITUTE_V1: &str = "/local/commitment/substitute";
    pub const LOCAL_PERCEIVED_STATE_V1: &str = "/local/perceived_state";
    pub const LOCAL_REQUEST_ADDRESS_V1: &str = "/local/request_address";
    pub const LOCAL_SIGN_PROOFS_V1: &str = "/local/sign_proofs";
    pub const LOCAL_SUBSTITUTE_V1: &str = "/local/substitute";
    pub const LOCAL_VALIDATE_ALPHA_LOCK_V1: &str = "/local/validate/alpha_lock";
    pub const LOCAL_VALIDATE_WALLET_LOCK_V1: &str = "/local/validate/wallet_lock";
    pub const LOCAL_VERIFY_EBILL_PAYMENT_V1: &str = "/local/verify_ebill_payment";
    pub const LOCAL_VERIFY_PAYMENT_V1: &str = "/local/verify_payment";
}

pub mod web_ep {
    pub const FOREIGN_ACTIVE_KEYSETS_V1: &str = "/v1/foreign/{pubkey}/active_keysets";
    pub const FOREIGN_ACTIVE_KEYSETS_V1_EXT: &str = "/v1/clowder/foreign/{pubkey}/active_keysets";
    pub const FOREIGN_OFFLINE_V1: &str = "/v1/foreign/offline/{pubkey}";
    pub const FOREIGN_OFFLINE_V1_EXT: &str = "/v1/clowder/foreign/offline/{pubkey}";
    pub const FOREIGN_PATH_V1: &str = "/v1/foreign/path";
    pub const FOREIGN_PATH_V1_EXT: &str = "/v1/clowder/foreign/path";
    pub const FOREIGN_STATUS_V1: &str = "/v1/foreign/status/{pubkey}";
    pub const FOREIGN_STATUS_V1_EXT: &str = "/v1/clowder/foreign/status/{pubkey}";
    pub const FOREIGN_SUBSTITUTE_V1: &str = "/v1/foreign/substitute/{pubkey}";
    pub const FOREIGN_SUBSTITUTE_V1_EXT: &str = "/v1/clowder/foreign/substitute/{pubkey}";
    pub const LOCAL_BETAS_V1: &str = "/v1/local/betas";
    pub const LOCAL_BETAS_V1_EXT: &str = "/v1/clowder/local/betas";
    pub const LOCAL_COVERAGE_V1: &str = "/v1/local/coverage";
    pub const LOCAL_COVERAGE_V1_EXT: &str = "/v1/clowder/local/coverage";
    pub const LOCAL_DERIVE_EBILL_PAYMENT_ADDRESS_V1: &str =
        "/v1/local/derive_ebill_payment_address";
    pub const LOCAL_DERIVE_EBILL_PAYMENT_ADDRESS_V1_EXT: &str =
        "/v1/clowder/local/derive_ebill_payment_address";
    pub const LOCAL_INFO_V1: &str = "/v1/local/info";
    pub const LOCAL_INFO_V1_EXT: &str = "/v1/clowder/local/info";
    pub const OFFLINE_EXCHANGE_V1: &str = "/v1/exchange/offline";
    pub const OFFLINE_EXCHANGE_V1_EXT: &str = "/v1/clowder/exchange/offline";
    pub const ONLINE_EXCHANGE_V1: &str = "/v1/exchange/online";
    pub const ONLINE_EXCHANGE_V1_EXT: &str = "/v1/clowder/exchange/online";
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

    pub fn get_base_url(&self) -> reqwest::Url {
        self.base.clone()
    }

    pub async fn get_alphas(&self) -> Result<wire_clowder::ConnectedMintsResponse> {
        let url = self
            .base
            .join(admin_ep::LOCAL_ALPHAS_V1)
            .expect("local alphas relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn get_mint_url(
        &self,
        node_id: &bitcoin::secp256k1::PublicKey,
    ) -> Result<clwdr_msgs::MintUrlResponse> {
        assert!(admin_ep::FOREIGN_URL_V1.contains("{pubkey}"));
        let path = admin_ep::FOREIGN_URL_V1.replace("{pubkey}", &node_id.to_string());
        let url = self.base.join(&path).expect("foreign url relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn post_sign_proofs(&self, proofs: &[Proof]) -> Result<clwdr_msgs::ProofsResponse> {
        let url = self
            .base
            .join(admin_ep::LOCAL_SIGN_PROOFS_V1)
            .expect("local sign proofs relative path");
        let response = self
            .cl
            .post(
                url,
                &clwdr_msgs::ProofsRequest {
                    proofs: proofs.to_vec(),
                },
            )
            .await?;
        Ok(response)
    }

    pub async fn post_validate_wallet_lock(
        &self,
        proofs: &[Proof],
    ) -> Result<clwdr_msgs::SuccessResponse> {
        let url = self
            .base
            .join(admin_ep::LOCAL_VALIDATE_WALLET_LOCK_V1)
            .expect("local validate wallet lock relative path");
        let response = self
            .cl
            .post(
                url,
                &clwdr_msgs::ProofsRequest {
                    proofs: proofs.to_vec(),
                },
            )
            .await?;
        Ok(response)
    }

    pub async fn post_validate_alpha_lock(
        &self,
        proofs: &[Proof],
    ) -> Result<clwdr_msgs::SuccessResponse> {
        let url = self
            .base
            .join(admin_ep::LOCAL_VALIDATE_ALPHA_LOCK_V1)
            .expect("local validate alpha lock relative path");
        let response = self
            .cl
            .post(
                url,
                &clwdr_msgs::ProofsRequest {
                    proofs: proofs.to_vec(),
                },
            )
            .await?;
        Ok(response)
    }

    #[allow(unused)]
    pub async fn post_checkstate(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
        keyset_ids: Vec<Id>,
        proof_ys: Vec<cashu::PublicKey>,
    ) -> Result<cashu::CheckStateResponse> {
        assert!(admin_ep::FOREIGN_CHECKSTATE_V1.contains("{pubkey}"));
        let req = clwdr_msgs::CheckStateRequest {
            ys: proof_ys,
            ids: keyset_ids,
        };
        let path = admin_ep::FOREIGN_CHECKSTATE_V1.replace("{pubkey}", &pubkey.to_string());
        let url = self
            .base
            .join(&path)
            .expect("foreign checkstate relative path");
        let response = self.cl.post(url, &req).await?;
        Ok(response)
    }

    pub async fn get_keyset(
        &self,
        alpha_id: &bitcoin::secp256k1::PublicKey,
        keyset_id: &Id,
    ) -> Result<KeysResponse> {
        assert!(admin_ep::FOREIGN_KEYS_V1.contains("{pubkey}"));
        assert!(admin_ep::FOREIGN_KEYS_V1.contains("{keyset_id}"));
        let path = admin_ep::FOREIGN_KEYS_V1
            .replace("{pubkey}", &alpha_id.to_string())
            .replace("{keyset_id}", &keyset_id.to_string());
        let url = self.base.join(&path).expect("foreign keys relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn post_commitment_substitute(
        &self,
        proofs: Vec<wire_keys::ProofFingerprint>,
        locks: Vec<Sha256>,
        wallet_pubkey: bitcoin::secp256k1::PublicKey,
    ) -> Result<bitcoin::secp256k1::schnorr::Signature> {
        let payload = wire_clowder::SubstituteExchangeRequest {
            proofs,
            locks,
            wallet_pubkey,
        };
        let url = self
            .base
            .join(admin_ep::LOCAL_COMMITMENT_SUBSTITUTE_V1)
            .expect("local commitment substitute relative path");
        let response: wire_clowder::SubstituteExchangeResponse =
            self.cl.post(url, &payload).await?;
        Ok(response.signature)
    }

    pub async fn get_keyset_info(
        &self,
        alpha_id: &bitcoin::secp256k1::PublicKey,
        keyset_id: &Id,
    ) -> Result<KeysetResponse> {
        assert!(admin_ep::FOREIGN_KEYSET_V1.contains("{pubkey}"));
        assert!(admin_ep::FOREIGN_KEYSET_V1.contains("{keyset_id}"));
        let path = admin_ep::FOREIGN_KEYSET_V1
            .replace("{pubkey}", &alpha_id.to_string())
            .replace("{keyset_id}", &keyset_id.to_string());
        let url = self.base.join(&path).expect("foreign keyset relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    #[allow(unused)]
    pub async fn post_determine_substitute_address(
        &self,
        mint_url: reqwest::Url,
    ) -> Result<clwdr_msgs::MintUrlResponse> {
        let url = self
            .base
            .join(admin_ep::LOCAL_SUBSTITUTE_V1)
            .expect("local substitute relative path");
        self.cl
            .post(url, &clwdr_msgs::MintUrlRequest { mint_url })
            .await
            .map_err(Into::into)
    }

    pub async fn get_mint_perceived_state(&self) -> Result<wire_clowder::PerceivedState> {
        let url = self
            .base
            .join(admin_ep::LOCAL_PERCEIVED_STATE_V1)
            .expect("local perceived state relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn post_verify_proofs(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
        proofs: Vec<Proof>,
    ) -> Result<clwdr_msgs::IntermintValidProofs> {
        assert!(admin_ep::FOREIGN_VERIFY_PROOFS_V1.contains("{pubkey}"));
        let path = admin_ep::FOREIGN_VERIFY_PROOFS_V1.replace("{pubkey}", &pubkey.to_string());
        let url = self
            .base
            .join(&path)
            .expect("foreign verify proofs relative path");
        self.cl
            .post(url, &clwdr_msgs::ProofsRequest { proofs })
            .await
            .map_err(Into::into)
    }

    pub async fn post_verify_fingerprints(
        &self,
        pubkey: &bitcoin::secp256k1::PublicKey,
        proofs: Vec<wire_keys::ProofFingerprint>,
    ) -> Result<clwdr_msgs::ValidFingerprints> {
        assert!(admin_ep::FOREIGN_VERIFY_FINGERPRINTS_V1.contains("{pubkey}"));
        let path =
            admin_ep::FOREIGN_VERIFY_FINGERPRINTS_V1.replace("{pubkey}", &pubkey.to_string());
        let url = self
            .base
            .join(&path)
            .expect("foreign verify fingerprints relative path");
        let response = self
            .cl
            .post(url, &clwdr_msgs::FingerprintRequest { proofs })
            .await?;
        Ok(response)
    }

    #[allow(unused)]
    pub async fn get_last_offline(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
    ) -> Result<clwdr_msgs::LastOfflineResponse> {
        assert!(admin_ep::FOREIGN_LAST_OFFLINE_V1.contains("{pubkey}"));
        let path = admin_ep::FOREIGN_LAST_OFFLINE_V1.replace("{pubkey}", &pubkey.to_string());
        let url = self
            .base
            .join(&path)
            .expect("foreign last offline relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    #[allow(unused)]
    pub async fn post_proofs_origin(
        &self,
        proofs: Vec<Proof>,
    ) -> Result<clwdr_msgs::IntermintOriginResponse> {
        let url = self
            .base
            .join(admin_ep::FOREIGN_PROOFS_ORIGIN_V1)
            .expect(" XXX relative path");
        self.cl
            .post(url, &clwdr_msgs::ProofsRequest { proofs })
            .await
            .map_err(Into::into)
    }

    pub async fn post_fingerprints_origin(
        &self,
        proofs: Vec<wire_keys::ProofFingerprint>,
    ) -> Result<clwdr_msgs::IntermintOriginResponse> {
        let url = self
            .base
            .join(admin_ep::FOREIGN_FINGERPRINTS_ORIGIN_V1)
            .expect("foreign fingerprints origin relative path");
        self.cl
            .post(url, &clwdr_msgs::FingerprintRequest { proofs })
            .await
            .map_err(Into::into)
    }

    pub async fn request_mint_address(
        &self,
        quote_id: uuid::Uuid,
        keyset_id: Id,
    ) -> Result<wire_clowder::OnchainAddressResponse> {
        let url = self
            .base
            .join(admin_ep::LOCAL_REQUEST_ADDRESS_V1)
            .expect("local request address relative path");
        let req = wire_clowder::OnchainAddressRequest {
            keyset_id,
            quote_id,
        };
        let response = self.cl.post(url, &req).await?;
        Ok(response)
    }

    pub async fn verify_mint_payment(
        &self,
        quote_id: uuid::Uuid,
        keyset_id: Id,
        min_confirmations: u32,
    ) -> Result<wire_clowder::VerifyMintPaymentResponse> {
        let url = self
            .base
            .join(admin_ep::LOCAL_VERIFY_PAYMENT_V1)
            .expect("local verify payment relative path");
        let req = wire_clowder::VerifyMintPaymentRequest {
            quote_id,
            keyset_id,
            min_confirmations,
        };
        let response = self.cl.post(url, &req).await?;
        Ok(response)
    }

    #[allow(unused)]
    pub async fn verify_ebill_payment(
        &self,
        bill_id: BillId,
    ) -> Result<wire_clowder::VerifyMintPaymentResponse> {
        let url = self
            .base
            .join(admin_ep::LOCAL_VERIFY_EBILL_PAYMENT_V1)
            .expect("local verify ebill payment relative path");
        let req = wire_clowder::VerifyEbillMintPaymentRequest { bill_id };
        let response = self.cl.post(url, &req).await?;
        Ok(response)
    }

    pub async fn get_collateral_onchain(
        &self,
        pubkey: &bitcoin::secp256k1::PublicKey,
    ) -> Result<wire_clowder::BitcoinAmountResponse> {
        assert!(admin_ep::FOREIGN_COLLATERAL_ONCHAIN_V1.contains("{pubkey}"));
        let path = admin_ep::FOREIGN_COLLATERAL_ONCHAIN_V1.replace("{pubkey}", &pubkey.to_string());
        let url = self
            .base
            .join(&path)
            .expect("foreign collateral onchain relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn get_collateral_ebill(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
    ) -> Result<wire_clowder::EbillAmountResponse> {
        assert!(admin_ep::FOREIGN_COLLATERAL_EBILL_V1.contains("{pubkey}"));
        let path = admin_ep::FOREIGN_COLLATERAL_EBILL_V1.replace("{pubkey}", &pubkey.to_string());
        let url = self
            .base
            .join(&path)
            .expect("foreign collateral ebill relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn get_collateral_eiou(
        &self,
        pubkey: &bitcoin::secp256k1::PublicKey,
    ) -> Result<wire_clowder::EiouAmountResponse> {
        assert!(admin_ep::FOREIGN_COLLATERAL_EIOU_V1.contains("{pubkey}"));
        let path = admin_ep::FOREIGN_COLLATERAL_EIOU_V1.replace("{pubkey}", &pubkey.to_string());
        let url = self
            .base
            .join(&path)
            .expect("foreign collateral eiou relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn get_circulating_supply(
        &self,
        pubkey: &bitcoin::secp256k1::PublicKey,
    ) -> Result<wire_clowder::SupplyResponse> {
        assert!(admin_ep::FOREIGN_CIRCULATING_SUPPLY_V1.contains("{pubkey}"));
        let path = admin_ep::FOREIGN_CIRCULATING_SUPPLY_V1.replace("{pubkey}", &pubkey.to_string());
        let url = self
            .base
            .join(&path)
            .expect("foreign circulating supply relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    #[allow(unused)]
    pub async fn get_keyset_mints(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
        keyset_id: &Id,
    ) -> Result<clwdr_msgs::AmountResponse> {
        assert!(admin_ep::FOREIGN_KEYSET_MINTS_V1.contains("{pubkey}"));
        assert!(admin_ep::FOREIGN_KEYSET_MINTS_V1.contains("{keyset_id}"));
        let path = admin_ep::FOREIGN_KEYSET_MINTS_V1
            .replace("{pubkey}", &pubkey.to_string())
            .replace("{keyset_id}", &keyset_id.to_string());
        let url = self
            .base
            .join(&path)
            .expect("foreign keyset mints relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    #[allow(unused)]
    pub async fn get_keyset_burns(
        &self,
        pubkey: bitcoin::secp256k1::PublicKey,
        keyset_id: &Id,
    ) -> Result<clwdr_msgs::AmountResponse> {
        assert!(admin_ep::FOREIGN_KEYSET_BURNS_V1.contains("{pubkey}"));
        assert!(admin_ep::FOREIGN_KEYSET_BURNS_V1.contains("{keyset_id}"));
        let path = admin_ep::FOREIGN_KEYSET_BURNS_V1
            .replace("{pubkey}", &pubkey.to_string())
            .replace("{keyset_id}", &keyset_id.to_string());
        let url = self
            .base
            .join(&path)
            .expect("foreign keyset burns relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn get_mint_collateral(&self) -> Result<wire_clowder::MintCollateralResponse> {
        let url = self
            .base
            .join(admin_ep::LOCAL_COLLATERAL_V1)
            .expect("local collateral relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn get_mint_circulating_supply(
        &self,
    ) -> Result<wire_clowder::MintCirculatingSupplyResponse> {
        let url = self
            .base
            .join(admin_ep::LOCAL_CIRCULATING_SUPPLY_V1)
            .expect("local circulating supply relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn fetch_mint_onchain_signatures(
        &self,
        alpha_id: &bitcoin::secp256k1::PublicKey,
        quote_id: &uuid::Uuid,
    ) -> Result<Option<Vec<crate::cashu::nuts::BlindSignature>>> {
        assert!(admin_ep::FOREIGN_MINT_ONCHAIN_SIGNATURES_V1.contains("{pubkey}"));
        assert!(admin_ep::FOREIGN_MINT_ONCHAIN_SIGNATURES_V1.contains("{quote_id}"));
        let path = admin_ep::FOREIGN_MINT_ONCHAIN_SIGNATURES_V1
            .replace("{pubkey}", &alpha_id.to_string())
            .replace("{quote_id}", &quote_id.to_string());
        let url = self
            .base
            .join(&path)
            .expect("foreign mint onchain signatures relative path");
        let response = self.cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn fetch_mint_onchain(
        &self,
        request: &crate::wire::mint::OnchainMintRequest,
    ) -> Result<crate::wire::mint::MintResponse> {
        let url = self
            .base
            .join(admin_ep::FOREIGN_MINT_ONCHAIN_V1)
            .expect("foreign mint onchain relative path");
        let response = self.cl.post(url, request).await?;
        Ok(response)
    }

    pub async fn protest_mint(
        &self,
        request: crate::wire::mint::MintProtestRequest,
    ) -> Result<crate::wire::mint::MintProtestResponse> {
        let url = self
            .base
            .join(admin_ep::FOREIGN_PROTEST_MINT_V1)
            .expect("foreign protest mint relative path");
        let response = self.cl.post(url, &request).await?;
        Ok(response)
    }

    pub async fn protest_swap(
        &self,
        request: crate::wire::swap::SwapProtestRequest,
    ) -> Result<crate::wire::swap::SwapProtestResponse> {
        let url = self
            .base
            .join(admin_ep::FOREIGN_PROTEST_SWAP_V1)
            .expect("foreign protest swap relative path");
        let response = self.cl.post(url, &request).await?;
        Ok(response)
    }

    pub async fn protest_melt(
        &self,
        request: crate::wire::melt::MeltProtestRequest,
    ) -> Result<crate::wire::melt::MeltProtestResponse> {
        let url = self
            .base
            .join(admin_ep::FOREIGN_PROTEST_MELT_V1)
            .expect("foreign protest melt relative path");
        let response = self.cl.post(url, &request).await?;
        Ok(response)
    }

    pub async fn get_info(&self) -> Result<wire_clowder::ClowderNodeInfo> {
        let response = common::get_info(&self.cl, &self.base, web_ep::LOCAL_INFO_V1).await?;
        Ok(response)
    }

    pub async fn get_betas(&self) -> Result<wire_clowder::ConnectedMintsResponse> {
        let response = common::get_betas(&self.cl, &self.base, web_ep::LOCAL_BETAS_V1).await?;
        Ok(response)
    }

    pub async fn get_substitute(
        &self,
        alpha_id: &bitcoin::secp256k1::PublicKey,
    ) -> Result<wire_clowder::ConnectedMintResponse> {
        let response = common::get_substitute(
            &self.cl,
            &self.base,
            web_ep::FOREIGN_SUBSTITUTE_V1,
            alpha_id,
        )
        .await?;
        Ok(response)
    }

    pub async fn get_offline(
        &self,
        pubkey: &bitcoin::secp256k1::PublicKey,
    ) -> Result<wire_clowder::OfflineResponse> {
        let response =
            common::get_offline(&self.cl, &self.base, web_ep::FOREIGN_OFFLINE_V1, pubkey).await?;
        Ok(response)
    }

    pub async fn get_status(
        &self,
        pubkey: &bitcoin::secp256k1::PublicKey,
    ) -> Result<wire_clowder::AlphaStateResponse> {
        let response =
            common::get_status(&self.cl, &self.base, web_ep::FOREIGN_STATUS_V1, pubkey).await?;
        Ok(response)
    }

    pub async fn derive_ebill_payment_address(
        &self,
        alpha_id: bitcoin::secp256k1::PublicKey,
        bill_id: BillId,
        block_id: u64,
        previous_block_hash: bitcoin::hashes::sha256::Hash,
    ) -> Result<wire_clowder::DeriveEbillPaymentAddressResponse> {
        let response = common::derive_ebill_payment_address(
            &self.cl,
            &self.base,
            web_ep::LOCAL_DERIVE_EBILL_PAYMENT_ADDRESS_V1,
            alpha_id,
            bill_id,
            block_id,
            previous_block_hash,
        )
        .await?;
        Ok(response)
    }

    pub async fn get_active_keysets(
        &self,
        alpha_id: &bitcoin::secp256k1::PublicKey,
    ) -> Result<cashu::KeysResponse> {
        let response = common::get_active_keysets(
            &self.cl,
            &self.base,
            web_ep::FOREIGN_ACTIVE_KEYSETS_V1,
            alpha_id,
        )
        .await?;
        Ok(response)
    }

    pub async fn post_online_exchange(
        &self,
        request: wire_exchange::OnlineExchangeRequest,
    ) -> Result<wire_exchange::OnlineExchangeResponse> {
        let response =
            common::post_online_exchange(&self.cl, &self.base, web_ep::ONLINE_EXCHANGE_V1, request)
                .await?;
        Ok(response)
    }

    pub async fn post_offline_exchange(
        &self,
        request: wire_exchange::OfflineExchangeRequest,
    ) -> Result<wire_exchange::OfflineExchangeResponse> {
        let response = common::post_offline_exchange(
            &self.cl,
            &self.base,
            web_ep::OFFLINE_EXCHANGE_V1,
            request,
        )
        .await?;
        Ok(response)
    }

    pub async fn post_path(
        &self,
        origin_mint_url: reqwest::Url,
    ) -> Result<wire_clowder::ConnectedMintsResponse> {
        let response = common::post_path(
            &self.cl,
            &self.base,
            web_ep::FOREIGN_PATH_V1,
            origin_mint_url,
        )
        .await?;
        Ok(response)
    }
}

pub(crate) mod common {
    use super::*;

    pub async fn get_info(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
    ) -> Result<wire_clowder::ClowderNodeInfo> {
        let url = base.join(ep).expect("info relative path");
        let response: wire_clowder::ClowderNodeInfo = cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn get_betas(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
    ) -> Result<wire_clowder::ConnectedMintsResponse> {
        let url = base.join(ep).expect("betas relative path");
        let response: wire_clowder::ConnectedMintsResponse = cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn get_active_keysets(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        alpha_id: &bitcoin::secp256k1::PublicKey,
    ) -> Result<KeysResponse> {
        assert!(ep.contains("{pubkey}"));
        let path = ep.replace("{pubkey}", &alpha_id.to_string());
        let url = base
            .join(&path)
            .expect("foreign active keysets relative path");
        let response = cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn get_substitute(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        alpha_id: &bitcoin::secp256k1::PublicKey,
    ) -> Result<wire_clowder::ConnectedMintResponse> {
        assert!(ep.contains("{pubkey}"));
        let path = ep.replace("{pubkey}", &alpha_id.to_string());
        let url = base.join(&path).expect("foreign substitute relative path");
        let response = cl.get(url, &[]).await?;
        Ok(response)
    }
    pub async fn get_offline(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        pubkey: &bitcoin::secp256k1::PublicKey,
    ) -> Result<wire_clowder::OfflineResponse> {
        assert!(ep.contains("{pubkey}"));
        let path = ep.replace("{pubkey}", &pubkey.to_string());
        let url = base.join(&path).expect("foreign offline relative path");
        let response = cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn get_status(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        pubkey: &bitcoin::secp256k1::PublicKey,
    ) -> Result<wire_clowder::AlphaStateResponse> {
        assert!(ep.contains("{pubkey}"));
        let path = ep.replace("{pubkey}", &pubkey.to_string());
        let url = base.join(&path).expect("foreign status relative path");
        let response = cl.get(url, &[]).await?;
        Ok(response)
    }

    pub async fn derive_ebill_payment_address(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        alpha_node_id: bitcoin::secp256k1::PublicKey,
        bill_id: BillId,
        block_id: u64,
        previous_block_hash: bitcoin::hashes::sha256::Hash,
    ) -> Result<wire_clowder::DeriveEbillPaymentAddressResponse> {
        let url = base
            .join(ep)
            .expect(" derive ebill payment address relative path");
        let req = wire_clowder::DeriveEbillPaymentAddressRequest {
            alpha_node_id,
            bill_id,
            block_id,
            previous_block_hash,
        };
        let response = cl.post(url, &req).await?;
        Ok(response)
    }

    pub async fn post_online_exchange(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        request: wire_exchange::OnlineExchangeRequest,
    ) -> Result<wire_exchange::OnlineExchangeResponse> {
        let url = base.join(ep).expect("online exchange relative path");
        let response: wire_exchange::OnlineExchangeResponse = cl.post(url, &request).await?;
        Ok(response)
    }

    pub async fn post_offline_exchange(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        request: wire_exchange::OfflineExchangeRequest,
    ) -> Result<wire_exchange::OfflineExchangeResponse> {
        let url = base.join(ep).expect("offline exchange relative path");
        let response: wire_exchange::OfflineExchangeResponse = cl.post(url, &request).await?;
        Ok(response)
    }

    pub async fn post_path(
        cl: &jsonrpc::Client,
        base: &reqwest::Url,
        ep: &'static str,
        origin_mint_url: reqwest::Url,
    ) -> Result<wire_clowder::ConnectedMintsResponse> {
        let url = base.join(ep).expect("foreign path relative path");
        let response = cl
            .post(url, &wire_clowder::PathRequest { origin_mint_url })
            .await?;
        Ok(response)
    }
}
