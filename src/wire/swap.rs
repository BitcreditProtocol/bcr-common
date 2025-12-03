// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::wire::{
    borsh::{
        deserialize_as_str, deserialize_vecof_cdkblindedmessage, serialize_as_str,
        serialize_vecof_cdkblindedmessage,
    },
    keys::ProofFingerprint,
};

// ----- end imports

///--------------------------- Burn tokens
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BurnRequest {
    pub proofs: Vec<cashu::Proof>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BurnResponse {
    pub ys: Vec<cashu::PublicKey>,
}

///--------------------------- Recover tokens
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecoverRequest {
    pub proofs: Vec<cashu::Proof>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecoverResponse {}

///--------------------------- Commitment Request

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct CommitmentContent {
    pub inputs: Vec<ProofFingerprint>,
    #[borsh(
        serialize_with = "serialize_vecof_cdkblindedmessage",
        deserialize_with = "deserialize_vecof_cdkblindedmessage"
    )]
    pub outputs: Vec<cashu::BlindedMessage>,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_as_str"
    )]
    pub expiration: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommitmentRequest {
    pub content: String, // base64, borsh-serialized CommitmentContent
}

///--------------------------- Commitment Response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommitmentResponse {
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}
