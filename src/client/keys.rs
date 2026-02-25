// ----- standard library imports
// ----- extra library imports
use thiserror::Error;
// ----- local imports
use crate::wire::keys as wire_keys;

// ----- end imports

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Error)]
pub enum Error {
    #[error("resource not found {0}")]
    KeysetIdNotFound(cashu::Id),
    #[error("mint operation not found {0}")]
    MintOpNotFound(uuid::Uuid),
    #[error("invalid request")]
    InvalidRequest,

    #[error("internal error {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("sign error {0}")]
    NUT20(#[from] cashu::nut20::Error),
}

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
    pub const KEYS_EP_V1: &'static str = "/v1/keys/{kid}";
    pub async fn keys(&self, kid: cashu::Id) -> Result<cashu::KeySet> {
        let url = self
            .base
            .join(&Self::KEYS_EP_V1.replace("{kid}", &kid.to_string()))
            .expect("keys relative path");
        let response = self.cl.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(kid));
        }
        let ks = response.json::<cashu::KeysResponse>().await?.keysets;
        ks.into_iter().next().ok_or(Error::KeysetIdNotFound(kid))
    }

    pub const LISTKEYS_EP_V1: &'static str = "/v1/keys";
    pub async fn list_keys(&self) -> Result<Vec<cashu::KeySet>> {
        let url = self
            .base
            .join(Self::LISTKEYS_EP_V1)
            .expect("list keys relative path");
        let response = self.cl.get(url).send().await?;
        let ks = response.json::<cashu::KeysResponse>().await?;
        Ok(ks.keysets)
    }

    pub const KEYSETINFO_EP_V1: &'static str = "/v1/keysets/{kid}";
    pub async fn keyset_info(&self, kid: cashu::Id) -> Result<cashu::KeySetInfo> {
        let url = self
            .base
            .join(&Self::KEYSETINFO_EP_V1.replace("{kid}", &kid.to_string()))
            .expect("keyset relative path");
        let response = self.cl.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(kid));
        }
        let ks = response.json::<cashu::KeySetInfo>().await?;
        Ok(ks)
    }

    pub const LISTKEYSETINFO_EP_V1: &'static str = "/v1/keysets";
    pub async fn list_keyset_info(&self) -> Result<Vec<cashu::KeySetInfo>> {
        let url = self
            .base
            .join(Self::LISTKEYSETINFO_EP_V1)
            .expect("keyset relative path");
        let response = self.cl.get(url).send().await?;
        let ks = response.json::<cashu::KeysetResponse>().await?;
        Ok(ks.keysets)
    }

    pub const SIGN_EP_V1: &'static str = "/v1/admin/keys/sign";
    pub async fn sign(&self, msg: &cashu::BlindedMessage) -> Result<cashu::BlindSignature> {
        let url = self
            .base
            .join(Self::SIGN_EP_V1)
            .expect("sign relative path");
        let request = self.cl.post(url).json(msg);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        if response.status() == reqwest::StatusCode::CONFLICT {
            return Err(Error::InvalidRequest);
        }
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(msg.keyset_id));
        }
        let sig = response.json::<cashu::BlindSignature>().await?;
        Ok(sig)
    }

    pub const VERIFY_PROOF_EP_V1: &'static str = "/v1/admin/keys/verify/proof";
    pub async fn verify_proof(&self, proof: &cashu::Proof) -> Result<()> {
        let url = self
            .base
            .join(Self::VERIFY_PROOF_EP_V1)
            .expect("verify relative path");
        let request = self.cl.post(url).json(proof);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(proof.keyset_id));
        }
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        response.error_for_status()?;
        Ok(())
    }

    pub const VERIFY_FINGERPRINT_EP_V1: &'static str = "/v1/admin/keys/verify/fingerprint";
    pub async fn verify_fingerprint(&self, fp: &wire_keys::ProofFingerprint) -> Result<()> {
        let url = self
            .base
            .join(Self::VERIFY_FINGERPRINT_EP_V1)
            .expect("verify relative path");
        let request = self.cl.post(url).json(fp);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(fp.keyset_id));
        }
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(Error::InvalidRequest);
        }
        response.error_for_status()?;
        Ok(())
    }

    pub const KEYSFOREXPIRATION_EP_V1: &'static str = "/v1/admin/keys/{date}";
    pub async fn keys_for_expiration(&self, date: chrono::NaiveDate) -> Result<cashu::Id> {
        let url = self
            .base
            .join(&Self::KEYSFOREXPIRATION_EP_V1.replace("{date}", &date.to_string()))
            .expect("keys for date relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        let kid = response.json::<cashu::Id>().await?;
        Ok(kid)
    }

    pub const NEWMINTOP_EP_V1: &'static str = "/v1/admin/keys/mintop";
    pub async fn new_mint_operation(
        &self,
        qid: uuid::Uuid,
        kid: cashu::Id,
        pk: cashu::PublicKey,
        target: cashu::Amount,
        bill_id: crate::core::BillId,
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
            bill_id,
        };
        let request = self.cl.post(url).json(&msg);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(kid));
        }
        let _response = response
            .json::<wire_keys::NewMintOperationResponse>()
            .await?;
        Ok(())
    }

    pub const MINTOPSTATUS_EP_V1: &'static str = "/v1/admin/keys/mintop/{qid}";
    pub async fn mint_operation_status(
        &self,
        qid: uuid::Uuid,
    ) -> Result<wire_keys::MintOperationStatus> {
        let url = self
            .base
            .join(&Self::MINTOPSTATUS_EP_V1.replace("{qid}", &qid.to_string()))
            .expect("mint operation status relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::MintOpNotFound(qid));
        }
        let response = response.json::<wire_keys::MintOperationStatus>().await?;
        Ok(response)
    }

    pub const LISTMINTOPS_EP_V1: &'static str = "/v1/admin/keys/mintops/{kid}";
    pub async fn list_mint_operations(&self, kid: cashu::Id) -> Result<Vec<uuid::Uuid>> {
        let url = self
            .base
            .join(&Self::LISTMINTOPS_EP_V1.replace("{kid}", &kid.to_string()))
            .expect("list mint operations relative path");
        let request = self.cl.get(url);
        let response = request.send().await?;
        let response = response.json::<Vec<uuid::Uuid>>().await?;
        Ok(response)
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
            return Err(Error::MintOpNotFound(qid));
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
    pub async fn deactivate_keyset(&self, kid: cashu::Id) -> Result<cashu::Id> {
        let url = self
            .base
            .join(Self::DEACTIVATEKEYSET_EP_V1)
            .expect("deactivate relative path");
        let msg = wire_keys::DeactivateKeysetRequest { kid };
        let request = self.cl.post(url).json(&msg);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::KeysetIdNotFound(kid));
        }
        let response: wire_keys::DeactivateKeysetResponse = response.json().await?;
        Ok(response.kid)
    }
}
