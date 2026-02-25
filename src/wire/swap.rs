// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::wire::{
    borsh::{
        deserialize_from_str, deserialize_vec_of_jsons, serialize_as_str, serialize_vec_of_jsons,
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
        serialize_with = "serialize_vec_of_jsons",
        deserialize_with = "deserialize_vec_of_jsons"
    )]
    pub outputs: Vec<cashu::BlindedMessage>,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AccumulatorWitness {
    pub root: [u8; 32],
    pub inputs: Vec<AccumulatorWitnessInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AccumulatorWitnessInput {
    pub leaf_index: usize,
    pub path: Vec<([u8; 32], bool)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StarkAccumulatorProof {
    pub root: [u8; 32],
    pub proof_bytes: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SwapRequest {
    pub inputs: Vec<cashu::Proof>,
    pub outputs: Vec<cashu::BlindedMessage>,
    pub accumulator_proof: Vec<u8>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SwapResponse {
    pub signatures: Vec<cashu::nuts::BlindSignature>,
    pub accumulator_witness: AccumulatorWitness,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MintWithWitnessResponse {
    pub signatures: Vec<cashu::nuts::BlindSignature>,
    pub accumulator_witness: AccumulatorWitness,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccumulatorInfoResponse {
    pub leaf_count: usize,
    pub root: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccumulatorPathsRequest {
    pub indices: Vec<usize>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccumulatorPathsResponse {
    pub root: String,
    pub paths: Vec<Option<Vec<(String, bool)>>>,
}
