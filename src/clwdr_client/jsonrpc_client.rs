// ----- standard library imports
// ----- extra library imports
use reqwest::Client as HttpClient;
use reqwest::Url;
use serde::{Serialize, de::DeserializeOwned};
// ----- project imports
use super::error::Result;
// ----- end imports

#[derive(Clone)]
pub struct JsonRpcClient {
    http: HttpClient,
}

impl JsonRpcClient {
    pub fn new() -> Self {
        let http = HttpClient::builder().build().unwrap();
        JsonRpcClient { http }
    }

    pub async fn get<T: DeserializeOwned>(&self, url: Url) -> Result<T> {
        let req = self.http.get(url);
        let resp = req.send().await?.error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn post<Req: Serialize, Res: DeserializeOwned>(
        &self,
        url: Url,
        body: &Req,
    ) -> Result<Res> {
        let req = self.http.post(url).json(body);
        let resp = req.send().await?.error_for_status()?;
        Ok(resp.json().await?)
    }

    pub async fn post_empty<Res: DeserializeOwned>(&self, url: Url) -> Result<Res> {
        let req = self.http.post(url);
        let resp = req.send().await?.error_for_status()?;
        Ok(resp.json().await?)
    }
}

impl Default for JsonRpcClient {
    fn default() -> Self {
        Self::new()
    }
}
