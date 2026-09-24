// ----- standard library imports
// ----- extra library imports
use bitcoin::secp256k1 as secp;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::{
    ecash,
    wire::{
        attestation::AttestedFingerprints,
        borsh::{deserialize_from_str, serialize_as_str},
        common::ProtestStatus,
    },
};

// ----- end imports

///--------------------------- Reserve tokens
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReserveRequest {
    #[schema(value_type = Vec<String>)]
    pub ys: Vec<secp::PublicKey>,
    #[serde(with = "time::serde::rfc3339")]
    pub deadline: time::OffsetDateTime,
}

///--------------------------- Burn tokens
#[derive(Debug, Serialize, Deserialize)]
pub struct BurnRequest {
    pub proofs: Vec<ecash::Proof>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BurnResponse {
    pub ys: Vec<secp::PublicKey>,
}

///--------------------------- Recover tokens
#[derive(Debug, Serialize, Deserialize)]
pub struct RecoverRequest {
    pub proofs: Vec<ecash::Proof>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecoverResponse {}

///--------------------------- Swap Commitment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct SwapCommitmentRequest {
    pub inputs: AttestedFingerprints,
    pub outputs: Vec<ecash::BlindedMessage>,
    pub expiry: u64,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub wallet_key: secp::PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SignedSwapCommitmentRequest {
    // serialized SwapCommitmentRequest
    pub payload: String,
    #[schema(value_type = String)]
    pub signature: bitcoin::secp256k1::schnorr::Signature,
}
///--------------------------- Swap Commitment Response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SwapCommitmentResponse {
    pub content: String,
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

///--------------------------- Swap Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRequest {
    pub inputs: Vec<ecash::Proof>,
    pub outputs: Vec<ecash::BlindedMessage>,
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

///--------------------------- Swap Response
#[derive(Debug, Serialize, Deserialize)]
pub struct SwapResponse {
    pub signatures: Vec<ecash::BlindSignature>,
}

///--------------------------- Swap Protest Request
#[derive(Debug, Serialize, Deserialize)]
pub struct SwapProtestRequest {
    pub alpha_id: bitcoin::secp256k1::PublicKey,
    pub proofs: Vec<ecash::Proof>,
    pub content: String,
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
    pub wallet_signature: bitcoin::secp256k1::schnorr::Signature,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blind_signatures: Option<Vec<ecash::BlindSignature>>,
}

///--------------------------- Swap Protest Response
#[derive(Debug, Serialize, Deserialize)]
pub struct SwapProtestResponse {
    pub status: ProtestStatus,
    pub signatures: Option<Vec<ecash::BlindSignature>>,
}
