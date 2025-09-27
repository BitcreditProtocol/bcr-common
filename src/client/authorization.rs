// ----- standard library imports
use std::sync::RwLock;
use thiserror::Error;
// ----- extra library imports
use reqwest::Url;
// ----- local modules

// ----- end imports

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("missing token url")]
    MissingTokenUrl,
    #[error("missing refresh token")]
    MissingRefreshToken,
    #[error("reqwest {0}")]
    Reqwest(#[from] reqwest::Error),
}

#[derive(Debug, Default)]
pub(crate) struct AuthorizationPlugin {
    token: RwLock<Option<String>>,
    refresh_token: RwLock<Option<String>>,
    token_url: RwLock<Option<Url>>,
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: String,
}

impl AuthorizationPlugin {
    pub(crate) async fn authenticate(
        &self,
        client: reqwest::Client,
        token_url: Url,
        client_id: &str,
        client_secret: &str,
        username: &str,
        password: &str,
    ) -> Result<std::time::Duration> {
        let resp: TokenResponse = client
            .post(token_url.clone())
            .form(&[
                ("grant_type", "password"),
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("username", username),
                ("password", password),
            ])
            .send()
            .await?
            .json()
            .await?;
        let TokenResponse {
            access_token,
            expires_in,
            refresh_token,
            ..
        } = resp;
        *self.token_url.write().unwrap() = Some(token_url);
        *self.token.write().unwrap() = Some(access_token);
        *self.refresh_token.write().unwrap() = Some(refresh_token);
        let expiration =
            std::time::Duration::from_secs(expires_in) - std::time::Duration::from_secs(5);
        Ok(expiration)
    }

    pub(crate) fn authorize(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let locked = self.token.read().unwrap();
        if let Some(token) = locked.as_ref() {
            request.bearer_auth(token)
        } else {
            request
        }
    }

    pub(crate) async fn refresh_access_token(
        &self,
        client: reqwest::Client,
        client_id: String,
    ) -> Result<std::time::Duration> {
        let Some(ref token_url) = *self.token_url.read().unwrap() else {
            return Err(Error::MissingTokenUrl);
        };
        let Some(refresh_token) = self.refresh_token.write().unwrap().take() else {
            return Err(Error::MissingRefreshToken);
        };
        let request = client.post(token_url.clone()).form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
            ("client_id", &client_id),
        ]);
        let response = request.send().await?;
        let token = response.json::<TokenResponse>().await?;
        let TokenResponse {
            access_token,
            expires_in,
            refresh_token,
            ..
        } = token;
        *self.token.write().unwrap() = Some(access_token);
        *self.refresh_token.write().unwrap() = Some(refresh_token);

        let expiration =
            std::time::Duration::from_secs(expires_in) - std::time::Duration::from_secs(5);
        Ok(expiration)
    }
}
