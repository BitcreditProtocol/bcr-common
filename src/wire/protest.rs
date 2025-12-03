// ----- standard library imports
// ----- extra library imports
use bitcoin::secp256k1;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::wire::borsh::{
    deserialize_vecof_cdkblindedmessage, deserialize_vecof_cdkproof,
    serialize_vecof_cdkblindedmessage, serialize_vecof_cdkproof,
};

// ----- end imports

/// users protest to any betas against alpha not providing commitment
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct ForeignSwapCommitmentRequest {
    pub original: crate::wire::swap::CommitmentRequest,
    pub mint: reqwest::Url,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub enum ForeignSwapCommitmentResponse {
    /// $\alpha$ accepted to sign off the request
    Accepted{
        #[schema(value_type = String)]
        commitment: secp256k1::schnorr::Signature,
    },
    /// the information provided by the wallet are flawed
    Denied {
        reason: String,
    },
    // the clowder has marked alpha as offline, beta substitute url is provided
    Offline {
        substitute: url::Url,
    },
    // the clowder has marked alpha as rabid, beta substitute url is provided
    Rabid {
        substitute: url::Url,
    }
}

/// users protest to any beta against alpha not honouring the commitment
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct SwapProtestRequest {
    pub original: crate::wire::swap::CommitmentRequest,
    #[schema(value_type = String)]
    pub commitment: secp256k1::schnorr::Signature,
    pub inputs: Vec<cashu::Proof>,
}

/// beta forces swap to alpha
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct SwapForceRequest {
    pub inputs: Vec<cashu::Proof>,
    pub outputs: Vec<cashu::BlindedMessage>,
    pub expiration: chrono::DateTime<chrono::Utc>,
    #[schema(value_type = String)]
    pub commitment: secp256k1::schnorr::Signature,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct SwapForceResponse {
    pub signatures: Vec<cashu::BlindSignature>,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub enum SwapProtestResponse {
    /// $\alpha$ accepted to sign off the request
    Accepted{
        #[schema(value_type = String)]
        commitment: secp256k1::schnorr::Signature,
    },
    /// the information provided by the wallet are flawed
    Denied {
        reason: String,
    },
    // the clowder has marked alpha as offline, beta substitute url is provided
    Offline {
        substitute: url::Url,
    },
    // the clowder has marked alpha as rabid, beta substitute url is provided
    Rabid {
        substitute: url::Url,
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct IntermintSwapContent {
    #[borsh(
        serialize_with = "serialize_vecof_cdkproof",
        deserialize_with = "deserialize_vecof_cdkproof"
    )]
    pub inputs: Vec<cashu::Proof>,
    #[borsh(
        serialize_with = "serialize_vecof_cdkblindedmessage",
        deserialize_with = "deserialize_vecof_cdkblindedmessage"
    )]
    pub outputs: Vec<cashu::BlindedMessage>,
}
