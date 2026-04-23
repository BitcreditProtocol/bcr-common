// ----- standard library imports
// ----- extra library imports
use reqwest::{StatusCode, Url};
use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;
// ----- local imports

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
}

#[derive(Debug, Clone)]
pub struct Client {
    cl: reqwest::Client,
}

impl Client {
    pub fn new() -> Self {
        let cl = reqwest::Client::new();
        Client { cl }
    }

    async fn to_error(response: reqwest::Response) -> Error {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        match status {
            StatusCode::NOT_FOUND => Error::ResourceNotFound(text),
            StatusCode::BAD_REQUEST => Error::InvalidRequest(text),
            _ => Error::Internal(text),
        }
    }

    pub async fn get<T: DeserializeOwned>(
        &self,
        url: Url,
        queries: &[(&'static str, String)],
    ) -> Result<T> {
        let request = self.cl.get(url).query(queries);
        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(Self::to_error(response).await);
        }
        let value: T = response.json().await?;
        Ok(value)
    }

    pub async fn post<Body: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        url: Url,
        body: &Body,
    ) -> Result<T> {
        let response = self.cl.post(url).json(body).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(Self::to_error(response).await);
        }
        let value: T = response.json().await?;
        Ok(value)
    }

    pub async fn post_empty<T: DeserializeOwned>(&self, url: Url) -> Result<T> {
        let response = self.cl.post(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(Self::to_error(response).await);
        }
        let value: T = response.json().await?;
        Ok(value)
    }

    pub async fn post_no_response<Body: Serialize + ?Sized>(
        &self,
        url: Url,
        body: &Body,
    ) -> Result<()> {
        let response = self.cl.post(url).json(body).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(Self::to_error(response).await);
        }
        Ok(())
    }

    pub async fn patch<Body: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        url: Url,
        body: &Body,
    ) -> Result<T> {
        let response = self.cl.patch(url).json(body).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(Self::to_error(response).await);
        }
        let value: T = response.json().await?;
        Ok(value)
    }

    pub async fn patch_no_response<Body: Serialize + ?Sized>(
        &self,
        url: Url,
        body: &Body,
    ) -> Result<()> {
        let response = self.cl.patch(url).json(body).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(Self::to_error(response).await);
        }
        Ok(())
    }

    pub async fn delete(&self, url: Url, queries: &[(&'static str, String)]) -> Result<()> {
        let request = self.cl.delete(url).query(queries);
        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(Self::to_error(response).await);
        }
        Ok(())
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
