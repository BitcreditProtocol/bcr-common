// ----- standard library imports
// ----- extra library imports
use bitcoin::secp256k1;
use thiserror::Error;
// ----- local imports
use crate::wire::{
    clowder::{self as wire_clowder, Coverage},
    exchange as wire_exchange,
};
// ----- end imports

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Error)]
pub enum Error {
    #[error("resource not found")]
    NotFound,
    #[error("internal error {0}")]
    Reqwest(#[from] reqwest::Error),
}

/// Clowder Wildcat client, for the Wallet to access Clowder endpoint on Wallet Aggregator
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

    pub const OFFLINE_EP_V1: &'static str = "/v1/foreign/offline/{alpha_id}";
    pub async fn get_offline(
        &self,
        alpha_id: secp256k1::PublicKey,
    ) -> Result<wire_clowder::OfflineResponse> {
        let url = self
            .base
            .join(&Self::OFFLINE_EP_V1.replace("{alpha_id}", &alpha_id.to_string()))
            .expect("offline relative path");
        let res = self.cl.get(url).send().await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::NotFound);
        }
        let response = res.json().await?;
        Ok(response)
    }

    pub const STATUS_EP_V1: &'static str = "/v1/foreign/status/{alpha_id}";
    pub async fn get_status(
        &self,
        alpha_id: secp256k1::PublicKey,
    ) -> Result<wire_clowder::AlphaStateResponse> {
        let url = self
            .base
            .join(&Self::STATUS_EP_V1.replace("{alpha_id}", &alpha_id.to_string()))
            .expect("status relative path");
        let res = self.cl.get(url).send().await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::NotFound);
        }
        let response = res.json().await?;
        Ok(response)
    }

    pub const SUBSTITUTE_EP_V1: &'static str = "/v1/foreign/substitute/{alpha_id}";
    pub async fn get_substitute(
        &self,
        alpha_id: secp256k1::PublicKey,
    ) -> Result<wire_clowder::ConnectedMintResponse> {
        let url = self
            .base
            .join(&Self::SUBSTITUTE_EP_V1.replace("{alpha_id}", &alpha_id.to_string()))
            .expect("substitute relative path");
        let res = self.cl.get(url).send().await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::NotFound);
        }
        let response = res.json().await?;
        Ok(response)
    }

    pub const KEYSETS_EP_V1: &'static str = "/v1/foreign/keysets/{alpha_id}";
    pub async fn get_active_keysets(
        &self,
        alpha_id: &secp256k1::PublicKey,
    ) -> Result<cashu::KeysResponse> {
        let url = self
            .base
            .join(&Self::KEYSETS_EP_V1.replace("{alpha_id}", &alpha_id.to_string()))
            .expect("keysets relative path");
        let res = self.cl.get(url).send().await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::NotFound);
        }
        let response = res.json().await?;
        Ok(response)
    }

    pub const PATH_EP_V1: &'static str = "/v1/local/path";
    pub async fn post_path(
        &self,
        origin_mint_url: cashu::MintUrl,
    ) -> Result<wire_clowder::ConnectedMintsResponse> {
        let url = self
            .base
            .join(Self::PATH_EP_V1)
            .expect("path relative path");
        let request = wire_clowder::PathRequest { origin_mint_url };
        let res = self.cl.post(url).json(&request).send().await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::NotFound);
        }
        let response = res.json().await?;
        Ok(response)
    }

    pub const ID_EP_V1: &'static str = "/v1/local/id";
    pub async fn get_id(&self) -> Result<wire_clowder::PublicKeyResponse> {
        let url = self.base.join(Self::ID_EP_V1).expect("id relative path");
        let res = self.cl.get(url).send().await?;
        let response = res.json().await?;
        Ok(response)
    }

    pub const BETAS_EP_V1: &'static str = "/v1/local/betas";
    pub async fn get_betas(&self) -> Result<wire_clowder::ConnectedMintsResponse> {
        let url = self
            .base
            .join(Self::BETAS_EP_V1)
            .expect("betas relative path");
        let res = self.cl.get(url).send().await?;
        let response = res.json().await?;
        Ok(response)
    }

    pub const COVERAGE_EP_V1: &'static str = "/v1/local/coverage";
    pub async fn post_coverage_exchange(&self) -> Result<Coverage> {
        let url = self
            .base
            .join(Self::COVERAGE_EP_V1)
            .expect("online exchange relative path");
        let res = self.cl.get(url).send().await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::NotFound);
        }
        let response = res.json().await?;
        Ok(response)
    }

    pub const ONLINE_EXCHANGE_EP_V1: &'static str = "/v1/exchange/online";
    pub async fn post_online_exchange(
        &self,
        request: wire_exchange::OnlineExchangeRequest,
    ) -> Result<wire_exchange::OnlineExchangeResponse> {
        let url = self
            .base
            .join(Self::ONLINE_EXCHANGE_EP_V1)
            .expect("online exchange relative path");
        let res = self.cl.post(url).json(&request).send().await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::NotFound);
        }
        let response = res.json().await?;
        Ok(response)
    }

    pub const OFFLINE_EXCHANGE_EP_V1: &'static str = "/v1/exchange/offline";
    pub async fn post_offline_exchange(
        &self,
        request: wire_exchange::OfflineExchangeRequest,
    ) -> Result<wire_exchange::OfflineExchangeResponse> {
        let url = self
            .base
            .join(Self::OFFLINE_EXCHANGE_EP_V1)
            .expect("offline exchange relative path");
        let res = self.cl.post(url).json(&request).send().await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::NotFound);
        }
        let response = res.json().await?;
        Ok(response)
    }
}
