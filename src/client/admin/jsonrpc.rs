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
    ResourceNotFound(serde_json::Value),
    #[error("invalid request {0}")]
    InvalidRequest(serde_json::Value),
    #[error("service unavailable {0}")]
    ServiceUnavailable(serde_json::Value),
    #[error("internal {0}")]
    Internal(String),
    #[error("internal error {0}")]
    Reqwest(#[from] reqwest::Error),
}

#[derive(Debug, Clone)]
pub struct Client {
    cl: reqwest::Client,
}

#[cfg(not(target_arch = "wasm32"))]
pub mod retry {
    pub type CachedEp = (&'static str, reqwest::Method);
    pub fn build_builder(
        base: &reqwest::Url,
        cached_eps: &'static [CachedEp],
        max_attempts: u32,
    ) -> reqwest::retry::Builder {
        let host = base.host().map(|h| h.to_string()).unwrap_or_default();
        reqwest::retry::for_host(host)
            .max_retries_per_request(max_attempts)
            .classify_fn(move |reqrep| {
                let ep = (reqrep.uri().path(), reqrep.method().clone());
                if cached_eps.contains(&ep) {
                    reqrep.retryable()
                } else {
                    reqrep.success()
                }
            })
    }
}

impl Client {
    pub fn new() -> Self {
        let cl = reqwest::Client::new();
        Client { cl }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_retry(builder: reqwest::retry::Builder) -> Self {
        let cl = reqwest::Client::builder()
            .retry(builder)
            .build()
            .expect("failed to build client with retry");
        Client { cl }
    }

    async fn to_error(response: reqwest::Response) -> Error {
        let status = response.status();
        match status {
            StatusCode::NOT_FOUND => {
                let value: serde_json::Value = match response.json().await {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!("failed to parse error response as json: {e}");
                        serde_json::Value::Null
                    }
                };
                Error::ResourceNotFound(value)
            }
            StatusCode::BAD_REQUEST => {
                let value: serde_json::Value = response.json().await.unwrap_or_default();
                Error::InvalidRequest(value)
            }
            StatusCode::SERVICE_UNAVAILABLE => {
                let value: serde_json::Value = response.json().await.unwrap_or_default();
                Error::ServiceUnavailable(value)
            }
            _ => {
                let text = response.text().await.unwrap_or_default();
                Error::Internal(text)
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    pub const EP_1: &str = "/test";
    pub const CACHED_EPS: [retry::CachedEp; 1] = [(EP_1, reqwest::Method::GET)];

    #[tokio::test]
    async fn test_retry_fail() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", EP_1)
            .with_status(500)
            .with_body("internal error")
            .expect(3)
            .create_async()
            .await;
        let base = Url::parse(&server.url()).unwrap();
        let url = base.join(EP_1).unwrap();
        let builder = retry::build_builder(&base, &CACHED_EPS, 3);
        let client = Client::with_retry(builder);
        let result: Result<()> = client.get(url, &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_retry_success_after_fail() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", EP_1)
            .with_status(500)
            .with_body("internal error")
            .expect(2)
            .create_async()
            .await;
        server
            .mock("GET", EP_1)
            .with_status(200)
            .with_body(r#""success""#)
            .expect(1)
            .create_async()
            .await;
        let base = Url::parse(&server.url()).unwrap();
        let url = base.join(EP_1).unwrap();
        let builder = retry::build_builder(&base, &CACHED_EPS, 3);
        let client = Client::with_retry(builder);
        let result: Result<String> = client.get(url, &[]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }
}
