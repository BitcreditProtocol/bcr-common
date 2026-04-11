// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::wire::{
    borsh::{
        deserialize_from_str, deserialize_vecof_blindedmessage, serialize_as_str,
        serialize_vecof_blindedmessage,
    },
    common::ProtestStatus,
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

///--------------------------- Swap Commitment Request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, BorshSerialize, BorshDeserialize)]
pub struct SwapCommitmentRequest {
    pub inputs: Vec<ProofFingerprint>,
    #[borsh(
        serialize_with = "serialize_vecof_blindedmessage",
        deserialize_with = "deserialize_vecof_blindedmessage"
    )]
    pub outputs: Vec<cashu::BlindedMessage>,
    pub expiry: u64,
    #[schema(value_type = String)]
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub wallet_key: cashu::PublicKey,
}

///--------------------------- Swap Commitment Response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SwapCommitmentResponse {
    pub content: String,
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

///--------------------------- Swap Request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SwapRequest {
    pub inputs: Vec<cashu::Proof>,
    pub outputs: Vec<cashu::BlindedMessage>,
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

///--------------------------- Swap Response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SwapResponse {
    pub signatures: Vec<cashu::BlindSignature>,
}

///--------------------------- Swap Protest Request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SwapProtestRequest {
    #[schema(value_type = String)]
    pub alpha_id: bitcoin::secp256k1::PublicKey,
    pub proofs: Vec<cashu::Proof>,
    pub content: String,
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
    #[schema(value_type = String)]
    pub wallet_signature: bitcoin::secp256k1::schnorr::Signature,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blind_signatures: Option<Vec<cashu::BlindSignature>>,
}

///--------------------------- Swap Protest Response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SwapProtestResponse {
    pub status: ProtestStatus,
    pub signatures: Option<Vec<cashu::BlindSignature>>,
}
