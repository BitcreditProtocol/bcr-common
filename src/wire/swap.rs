// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::wire::borsh::{
    deserialize_as_str, deserialize_vec_of_jsons, deserialize_vec_of_strs, serialize_as_str,
    serialize_vec_of_jsons, serialize_vec_of_strs,
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
    #[borsh(
        serialize_with = "serialize_vec_of_strs",
        deserialize_with = "deserialize_vec_of_strs"
    )]
    pub proofs_fp: Vec<cashu::PublicKey>,
    #[borsh(
        serialize_with = "serialize_vec_of_jsons",
        deserialize_with = "deserialize_vec_of_jsons"
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

///--------------------------- Protest
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Protest {
    pub commitment_content: String, // base64, borsh-serialized CommitmentContent
    #[schema(value_type = String)]
    pub signature: bitcoin::secp256k1::schnorr::Signature,
    pub proofs: Vec<cashu::Proof>,
}

///--------------------------- ForceRequest
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ForceSwapRequest {
    pub inputs: Vec<cashu::Proof>,
    pub outputs: Vec<cashu::BlindedMessage>,
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}
