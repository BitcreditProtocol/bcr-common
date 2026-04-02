// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::wire::{
    borsh::{
        deserialize_from_str, deserialize_option_vecof_blindsignature,
        deserialize_vecof_blindedmessage, deserialize_vecof_cdkproof, serialize_as_str,
        serialize_option_vecof_blindsignature, serialize_vecof_blindedmessage,
        serialize_vecof_cdkproof,
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

///--------------------------- Swap Commitment Request Body
#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct SwapCommitmentRequestBody {
    pub inputs: Vec<ProofFingerprint>,
    #[borsh(
        serialize_with = "serialize_vecof_blindedmessage",
        deserialize_with = "deserialize_vecof_blindedmessage"
    )]
    pub outputs: Vec<cashu::BlindedMessage>,
    pub expiry_height: u64,
}

///--------------------------- Swap Commitment Request
#[derive(Debug, Serialize, Deserialize, ToSchema, BorshSerialize, BorshDeserialize)]
pub struct SwapCommitmentRequest {
    pub content: String, // base64(borsh(SwapCommitmentRequestBody)), the swap body
    #[schema(value_type = String)]
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub wallet_key: cashu::PublicKey,
    #[schema(value_type = String)]
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub wallet_signature: bitcoin::secp256k1::schnorr::Signature,
}

///--------------------------- Swap Commitment Response
/// The mint borsh-serializes the full SwapCommitmentRequest (body + wallet fields),
/// signs that, and returns the serialized bytes as `content` with the signature as `commitment`.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SwapCommitmentResponse {
    pub content: String, // base64(borsh(SwapCommitmentRequest)), signed by mint
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

///--------------------------- Swap Protest Request Body
#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct SwapProtestRequestBody {
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub alpha_id: bitcoin::secp256k1::PublicKey,
    #[borsh(
        serialize_with = "serialize_vecof_cdkproof",
        deserialize_with = "deserialize_vecof_cdkproof"
    )]
    pub proofs: Vec<cashu::Proof>,
    pub content: String, // base64(borsh(SwapCommitmentRequestBody)), same as SwapCommitmentRequest.content
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
    #[borsh(
        serialize_with = "serialize_option_vecof_blindsignature",
        deserialize_with = "deserialize_option_vecof_blindsignature"
    )]
    pub blind_signatures: Option<Vec<cashu::BlindSignature>>,
}

///--------------------------- Swap Protest Request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SwapProtestRequest {
    pub body: String, // base64, borsh-serialized SwapProtestRequestBody
    #[schema(value_type = String)]
    pub wallet_key: cashu::PublicKey,
    #[schema(value_type = String)]
    pub wallet_signature: bitcoin::secp256k1::schnorr::Signature,
}

///--------------------------- Swap Protest Response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SwapProtestResponse {
    pub status: ProtestStatus,
    pub signatures: Option<Vec<cashu::BlindSignature>>,
}
