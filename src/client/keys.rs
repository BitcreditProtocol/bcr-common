// ----- standard library imports
// ----- extra library imports
use thiserror::Error;
// ----- local imports
#[cfg(feature = "authorized")]
use crate::wire::keys::{
    DeactivateKeysetRequest, DeactivateKeysetResponse, EnableKeysetRequest, EnableKeysetResponse,
    GenerateKeysetRequest, KeysetMintCondition, PreSignRequest,
};

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
                self.cl.clone(),
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
    pub async fn refresh_access_token(
        &self,
        client: reqwest::Client,
        client_id: String,
    ) -> Result<std::time::Duration> {
        let exp = self.auth.refresh_access_token(client, client_id).await?;
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
        let ks = res.json::<cashu::KeySet>().await?;
        Ok(ks)
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
        let url = self.base.join(Self::SIGN_EP).expect("sign relative path");
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

    pub const VERIFY_EP_V1: &'static str = "/v1/admin/keys/verify";
    #[cfg(feature = "authorized")]
    pub async fn verify(&self, proof: &cashu::Proof) -> Result<()> {
        let url = self
            .base
            .join(Self::VERIFY_EP)
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

    pub const PRESIGN_EP_V1: &'static str = "/v1/admin/keys/pre_sign";
    #[cfg(feature = "authorized")]
    pub async fn pre_sign(
        &self,
        qid: uuid::Uuid,
        msg: &cashu::BlindedMessage,
    ) -> Result<cashu::BlindSignature> {
        let url = self
            .base
            .join(Self::PRESIGN_EP_V1)
            .expect("pre_sign relative path");
        let msg = PreSignRequest {
            qid,
            msg: msg.clone(),
        };
        let request = self.cl.post(url).json(&msg);
        let response = self.auth.authorize(request).send().await?;
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        let sig = response.json::<cashu::BlindSignature>().await?;
        Ok(sig)
    }

    pub const GENERATEKEYSET_EP_V1: &'static str = "/v1/admin/keys/generate";
    #[cfg(feature = "authorized")]
    pub async fn generate_keyset(
        &self,
        qid: uuid::Uuid,
        amount: cashu::Amount,
        public_key: cashu::PublicKey,
        expire: chrono::DateTime<chrono::Utc>,
    ) -> Result<cashu::Id> {
        let url = self
            .base
            .join(Self::GENERATEKEYSET_EP_V1)
            .expect("generate relative path");
        let msg = GenerateKeysetRequest {
            qid,
            condition: KeysetMintCondition { amount, public_key },
            expire,
        };
        let request = self.cl.post(url).json(&msg);
        let response = self.auth.authorize(request).send().await?;
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        let kid = response.json::<cashu::Id>().await?;
        Ok(kid)
    }

    pub const ENABLEKEYSET_EP_V1: &'static str = "/v1/admin/keys/enable";
    #[cfg(feature = "authorized")]
    pub async fn enable_keyset(&self, qid: uuid::Uuid) -> Result<cashu::Id> {
        let url = self
            .base
            .join(Self::ENABLEKEYSET_EP_V1)
            .expect("enable relative path");
        let msg = EnableKeysetRequest { qid };
        let request = self.cl.post(url).json(&msg);
        let response = self.auth.authorize(request).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceFromIdNotFound(qid));
        }
        let response: EnableKeysetResponse = response.json().await?;
        Ok(response.kid)
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
        let msg = DeactivateKeysetRequest { kid };
        let request = self.cl.post(url).json(&msg);
        let response = self.auth.authorize(request).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::ResourceNotFound(kid));
        }
        let response: DeactivateKeysetResponse = response.json().await?;
        Ok(response.kid)
    }
}

#[cfg(feature = "test-utils")]
pub mod test_utils {
    use super::*;

    #[derive(Debug, Default, Clone)]
    pub struct KeyClient {
        pub keys: bcr_wdc_key_service::test_utils::InMemoryRepository,
    }

    impl Client {
        pub async fn keyset(&self, kid: cashu::Id) -> Result<cashu::KeySet> {
            let res = self.keys.keyset(&kid).expect("InMemoryRepository");
            res.ok_or(Error::ResourceNotFound(kid))
                .map(std::convert::Into::into)
        }
        pub async fn list_keyset(&self) -> Result<Vec<cashu::KeySet>> {
            let res = self.keys.list_keyset().expect("InMemoryRepository");
            let ret = res.into_iter().map(cashu::KeySet::from).collect();
            Ok(ret)
        }
        pub async fn keyset_info(&self, kid: cashu::Id) -> Result<cashu::KeySetInfo> {
            self.keys
                .info(&kid)
                .expect("InMemoryRepository")
                .ok_or(Error::ResourceNotFound(kid))
                .map(std::convert::Into::into)
        }
        pub async fn list_keyset_info(&self) -> Result<Vec<cashu::KeySetInfo>> {
            let res = self.keys.list_info().expect("InMemoryRepository");
            let ret = res.into_iter().map(cashu::KeySetInfo::from).collect();
            Ok(ret)
        }
        pub async fn sign(&self, msg: &cashu::BlindedMessage) -> Result<cashu::BlindSignature> {
            let res = self
                .keys
                .keyset(&msg.keyset_id)
                .expect("InMemoryRepository");
            let keys = res.ok_or(Error::ResourceNotFound(msg.keyset_id))?;
            bcr_wdc_utils::keys::sign_with_keys(&keys, msg).map_err(|_| Error::InvalidRequest)
        }
        pub async fn verify(&self, proof: &cashu::Proof) -> Result<bool> {
            let res = self
                .keys
                .keyset(&proof.keyset_id)
                .expect("InMemoryRepository");
            let keys = res.ok_or(Error::ResourceNotFound(proof.keyset_id))?;
            bcr_wdc_utils::keys::verify_with_keys(&keys, proof)
                .map_err(|_| Error::InvalidRequest)?;
            Ok(true)
        }
        pub async fn pre_sign(
            &self,
            _qid: uuid::Uuid,
            _msg: &cashu::BlindedMessage,
        ) -> Result<cashu::BlindSignature> {
            todo!()
        }

        pub async fn generate_keyset(
            &self,
            _qid: uuid::Uuid,
            _target: cashu::Amount,
            _pub_key: cashu::PublicKey,
            _expire: chrono::DateTime<chrono::Utc>,
        ) -> Result<cashu::Id> {
            todo!();
        }

        pub async fn mint(
            &self,
            _outputs: &[cashu::BlindedMessage],
            _sk: cashu::SecretKey,
        ) -> Result<()> {
            todo!()
        }

        pub async fn restore(
            &self,
            _outputs: Vec<cashu::BlindedMessage>,
        ) -> Result<Vec<(cashu::BlindedMessage, cashu::BlindSignature)>> {
            todo!()
        }
    }
}
