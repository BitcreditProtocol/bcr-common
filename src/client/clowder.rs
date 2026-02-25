// ----- standard library imports
// ----- extra library imports
use bitcoin::secp256k1;
use thiserror::Error;
// ----- local imports
use crate::wire::{
    clowder::{self as wire_clowder, ClowderNodeInfo, Coverage},
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

    pub const FOREIGN_OFFLINE_EP_V1: &'static str = "/v1/foreign/offline/{alpha_id}";
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
            return Err(Error::NotFound);
        }
        let payload = response.json().await?;
        Ok(payload)
    }

    pub const FOREIGN_STATUS_EP_V1: &'static str = "/v1/foreign/status/{alpha_id}";
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
            return Err(Error::NotFound);
        }
        let response = response.json().await?;
        Ok(response)
    }

    pub const FOREIGN_SUBSTITUTE_EP_V1: &'static str = "/v1/foreign/substitute/{alpha_id}";
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
            return Err(Error::NotFound);
        }
        let response = response.json().await?;
        Ok(response)
    }

    pub const FOREIGN_KEYSETS_EP_V1: &'static str = "/v1/foreign/keysets/{alpha_id}";
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
            return Err(Error::NotFound);
        }
        let response = response.json().await?;
        Ok(response)
    }

    pub const LOCAL_PATH_EP_V1: &'static str = "/v1/local/path";
    pub async fn post_path(
        &self,
        origin_mint_url: cashu::MintUrl,
    ) -> Result<wire_clowder::ConnectedMintsResponse> {
        let url = self
            .base
            .join(Self::LOCAL_PATH_EP_V1)
            .expect("path relative path");
        let request = wire_clowder::PathRequest { origin_mint_url };
        let response = self.cl.post(url).json(&request).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::NotFound);
        }
        let response = response.json().await?;
        Ok(response)
    }

    pub const LOCAL_INFO_EP_V1: &'static str = "/v1/local/info";
    pub async fn get_info(&self) -> Result<ClowderNodeInfo> {
        let url = self
            .base
            .join(Self::LOCAL_INFO_EP_V1)
            .expect("info relative path");
        let response = self.cl.get(url).send().await?;
        let response = response.json().await?;
        Ok(response)
    }

    pub const LOCAL_BETAS_EP_V1: &'static str = "/v1/local/betas";
    pub async fn get_betas(&self) -> Result<wire_clowder::ConnectedMintsResponse> {
        let url = self
            .base
            .join(Self::LOCAL_BETAS_EP_V1)
            .expect("betas relative path");
        let response = self.cl.get(url).send().await?;
        let response = response.json().await?;
        Ok(response)
    }

    pub const LOCAL_COVERAGE_EP_V1: &'static str = "/v1/local/coverage";
    pub async fn get_coverage(&self) -> Result<Coverage> {
        let url = self
            .base
            .join(Self::LOCAL_COVERAGE_EP_V1)
            .expect("coverage relative path");
        let response = self.cl.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::NotFound);
        }
        let response = response.json().await?;
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
        let response = self.cl.post(url).json(&request).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::NotFound);
        }
        let response = response.json().await?;
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
        let response = self.cl.post(url).json(&request).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::NotFound);
        }
        let response = response.json().await?;
        Ok(response)
    }
}
