// ----- standard library imports
// ----- extra library imports
use thiserror::Error;
// ----- local imports
#[cfg(feature = "authorized")]
use crate::wire::keys as wire_keys;

// ----- end imports

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Error)]
pub enum Error {
    #[error("resource not found {0}")]
    ResourceNotFound(cashu::Id),
    #[error("resource from id not found {0}")]
    ResourceFromIdNotFound(uuid::Uuid),
    #[error("invalid request")]
    InvalidRequest,
    #[error("authorization {0}")]
    Auth(String),

    #[error("internal error {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("sign error {0}")]
    NUT20(#[from] cashu::nut20::Error),
}

#[cfg(feature = "authorized")]
impl std::convert::From<crate::client::authorization::Error> for Error {
    fn from(e: crate::client::authorization::Error) -> Self {
        match e {
            crate::client::authorization::Error::Reqwest(e) => Error::Reqwest(e),
            _ => Error::Auth(e.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    cl: reqwest::Client,
    base: reqwest::Url,
    #[cfg(feature = "authorized")]
    auth: std::sync::Arc<crate::client::authorization::AuthorizationPlugin>,
}

impl Client {
    pub fn new(base: reqwest::Url) -> Self {
        Self {
            cl: reqwest::Client::new(),
            base,
            #[cfg(feature = "authorized")]
            auth: Default::default(),
        }
    }

    #[cfg(feature = "authorized")]
    pub async fn authenticate(
        &mut self,
        token_url: reqwest::Url,
        client_id: &str,
        client_secret: &str,
        username: &str,
        password: &str,
    ) -> Result<std::time::Duration> {
        let exp = self
            .auth
            .authenticate(
                &self.cl,
                token_url,
                client_id,
                client_secret,
                username,
                password,
            )
            .await?;
        Ok(exp)
    }

    #[cfg(feature = "authorized")]
    pub async fn refresh_access_token(&self, client_id: String) -> Result<std::time::Duration> {
        let exp = self.auth.refresh_access_token(&self.cl, client_id).await?;
        Ok(exp)
    }

    pub const KEYS_EP_V1: &'static str = "/v1/keys/{kid}";
    pub async fn keys(&self, kid: cashu::Id) -> Result<cashu::KeySet> {
        let url = self
            .base
            .join(&Self::KEYS_EP_V1.replace("{kid}", &kid.to_string()))
            .expect("keys relative path");
        let res = self.cl.get(url).send().await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(kid));
        }
        let ks = res.json::<cashu::KeysResponse>().await?.keysets;
        ks.into_iter().next().ok_or(Error::ResourceNotFound(kid))
    }

    pub const LISTKEYS_EP_V1: &'static str = "/v1/keys";
    pub async fn list_keys(&self) -> Result<Vec<cashu::KeySet>> {
        let url = self
            .base
            .join(Self::LISTKEYS_EP_V1)
            .expect("list keys relative path");
        let res = self.cl.get(url).send().await?;
        let ks = res.json::<cashu::KeysResponse>().await?;
        Ok(ks.keysets)
    }

    pub const KEYSETINFO_EP_V1: &'static str = "/v1/keysets/{kid}";
    pub async fn keyset_info(&self, kid: cashu::Id) -> Result<cashu::KeySetInfo> {
        let url = self
            .base
            .join(&Self::KEYSETINFO_EP_V1.replace("{kid}", &kid.to_string()))
            .expect("keyset relative path");
        let res = self.cl.get(url).send().await?;
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(kid));
        }
        let ks = res.json::<cashu::KeySetInfo>().await?;
        Ok(ks)
    }

    pub const LISTKEYSETINFO_EP_V1: &'static str = "/v1/keysets";
    pub async fn list_keyset_info(&self) -> Result<Vec<cashu::KeySetInfo>> {
        let url = self
            .base
            .join(Self::LISTKEYSETINFO_EP_V1)
            .expect("keyset relative path");
        let res = self.cl.get(url).send().await?;
        let ks = res.json::<cashu::KeysetResponse>().await?;
        Ok(ks.keysets)
    }

    pub const SIGN_EP_V1: &'static str = "/v1/admin/keys/sign";
    #[cfg(feature = "authorized")]
    pub async fn sign(&self, msg: &cashu::BlindedMessage) -> Result<cashu::BlindSignature> {
        let url = self
            .base
            .join(Self::SIGN_EP_V1)
            .expect("sign relative path");
        let request = self.cl.post(url).json(msg);
        let response = self.auth.authorize(request).send().await?;
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        if response.status() == reqwest::StatusCode::CONFLICT {
            return Err(Error::InvalidRequest);
        }
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(msg.keyset_id));
        }
        let sig = response.json::<cashu::BlindSignature>().await?;
        Ok(sig)
    }

    pub const VERIFY_PROOF_EP_V1: &'static str = "/v1/admin/keys/verify/proof";
    #[cfg(feature = "authorized")]
    pub async fn verify_proof(&self, proof: &cashu::Proof) -> Result<()> {
        let url = self
            .base
            .join(Self::VERIFY_PROOF_EP_V1)
            .expect("verify relative path");
        let request = self.cl.post(url).json(proof);
        let response = self.auth.authorize(request).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(proof.keyset_id));
        }
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        response.error_for_status()?;
        Ok(())
    }

    pub const VERIFY_FINGERPRINT_EP_V1: &'static str = "/v1/admin/keys/verify/fingerprint";
    #[cfg(feature = "authorized")]
    pub async fn verify_fingerprint(&self, fp: &wire_keys::ProofFingerprint) -> Result<()> {
        let url = self
            .base
            .join(Self::VERIFY_FINGERPRINT_EP_V1)
            .expect("verify relative path");
        let request = self.cl.post(url).json(fp);
        let response = self.auth.authorize(request).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(fp.keyset_id));
        }
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        response.error_for_status()?;
        Ok(())
    }


    pub const KEYSFOREXPIRATION_EP_V1: &'static str = "/v1/admin/keys/{date}";
    #[cfg(feature = "authorized")]
    pub async fn keys_for_expiration(&self, date: chrono::NaiveDate) -> Result<cashu::Id> {
        let url = self
            .base
            .join(&Self::KEYSFOREXPIRATION_EP_V1.replace("{date}", &date.to_string()))
            .expect("keys for date relative path");
        let request = self.cl.get(url);
        let res = self.auth.authorize(request).send().await?;
        let kid = res.json::<cashu::Id>().await?;
        Ok(kid)
    }

    pub const NEWMINTOP_EP_V1: &'static str = "/v1/admin/keys/mintop";
    #[cfg(feature = "authorized")]
    pub async fn new_mint_operation(
        &self,
        qid: uuid::Uuid,
        kid: cashu::Id,
        pk: cashu::PublicKey,
        target: cashu::Amount,
    ) -> Result<()> {
        let url = self
            .base
            .join(Self::NEWMINTOP_EP_V1)
            .expect("mint operation relative path");
        let msg = wire_keys::NewMintOperationRequest {
            quote_id: qid,
            kid,
            pub_key: pk,
            target,
        };
        let result = self.cl.post(url).json(&msg).send().await?;
        if result.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(kid));
        }
        let _response = result.json::<wire_keys::NewMintOperationResponse>().await?;
        Ok(())
    }

    pub const MINT_EP_V1: &'static str = "/v1/mint/ebill";
    pub async fn mint(
        &self,
        qid: uuid::Uuid,
        outputs: Vec<cashu::BlindedMessage>,
        sk: cashu::SecretKey,
    ) -> Result<Vec<cashu::BlindSignature>> {
        let url = self
            .base
            .join(Self::MINT_EP_V1)
            .expect("mint relative path");
        let mut msg = cashu::MintRequest {
            quote: qid,
            outputs,
            signature: None,
        };
        msg.sign(sk)?;
        let result = self.cl.post(url).json(&msg).send().await?;
        if result.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceFromIdNotFound(qid));
        }
        let response = result.json::<cashu::MintResponse>().await?;
        Ok(response.signatures)
    }

    pub const RESTORE_EP_V1: &'static str = "/v1/restore";
    pub async fn restore(
        &self,
        outputs: Vec<cashu::BlindedMessage>,
    ) -> Result<Vec<(cashu::BlindedMessage, cashu::BlindSignature)>> {
        let url = self
            .base
            .join(Self::RESTORE_EP_V1)
            .expect("restore relative path");
        let msg = cashu::RestoreRequest { outputs };
        let response = self.cl.post(url).json(&msg).send().await?;
        let msg: cashu::RestoreResponse = response.json().await?;
        let cashu::RestoreResponse {
            outputs,
            signatures,
            ..
        } = msg;
        let ret_val = outputs
            .into_iter()
            .zip(signatures.into_iter())
            .collect::<Vec<_>>();
        Ok(ret_val)
    }

    pub const DEACTIVATEKEYSET_EP_V1: &'static str = "/v1/admin/keys/deactivate";
    #[cfg(feature = "authorized")]
    pub async fn deactivate_keyset(&self, kid: cashu::Id) -> Result<cashu::Id> {
        let url = self
            .base
            .join(Self::DEACTIVATEKEYSET_EP_V1)
            .expect("deactivate relative path");
        let msg = wire_keys::DeactivateKeysetRequest { kid };
        let request = self.cl.post(url).json(&msg);
        let response = self.auth.authorize(request).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(kid));
        }
        let response: wire_keys::DeactivateKeysetResponse = response.json().await?;
        Ok(response.kid)
    }
}
