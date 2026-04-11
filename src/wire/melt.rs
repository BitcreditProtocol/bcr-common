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

///--------------------------- Melt Quote Onchain Request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, BorshSerialize, BorshDeserialize)]
pub struct MeltQuoteOnchainRequest {
    pub inputs: Vec<ProofFingerprint>,
    pub address: String,
    pub amount: u64,
    #[borsh(
        serialize_with = "serialize_vecof_blindedmessage",
        deserialize_with = "deserialize_vecof_blindedmessage"
    )]
    pub change: Vec<cashu::BlindedMessage>,
    #[schema(value_type = String)]
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub wallet_key: cashu::PublicKey,
}

///--------------------------- Melt Quote Onchain Response Body
#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct MeltQuoteOnchainResponseBody {
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub quote: uuid::Uuid,
    pub inputs: Vec<ProofFingerprint>,
    pub address: String,
    pub amount: u64,
    #[borsh(
        serialize_with = "serialize_vecof_blindedmessage",
        deserialize_with = "deserialize_vecof_blindedmessage"
    )]
    pub change: Vec<cashu::BlindedMessage>,
    /// Unix timestamp when the commitment expires
    pub expiry: u64,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub wallet_key: cashu::PublicKey,
}

///--------------------------- Melt Quote Onchain Response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltQuoteOnchainResponse {
    pub content: String,
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

///--------------------------- Melt Tx
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltTx {
    #[schema(value_type = Option<String>)]
    pub alpha_txid: Option<bitcoin::Txid>,
    #[schema(value_type = Option<String>)]
    pub beta_txid: Option<bitcoin::Txid>,
}

///--------------------------- Melt Onchain Request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltOnchainRequest {
    #[schema(value_type = String)]
    pub quote: uuid::Uuid,
    pub inputs: Vec<cashu::Proof>,
}

///--------------------------- Melt Onchain Response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltOnchainResponse {
    pub txid: MeltTx,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub change: Vec<cashu::BlindSignature>,
}

///--------------------------- Melt Protest Request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltProtestRequest {
    #[schema(value_type = String)]
    pub alpha_id: bitcoin::secp256k1::PublicKey,
    #[schema(value_type = String)]
    pub quote_id: uuid::Uuid,
    pub content: String,
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
    #[schema(value_type = String)]
    pub wallet_signature: bitcoin::secp256k1::schnorr::Signature,
}

///--------------------------- Melt Protest Response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltProtestResponse {
    pub status: ProtestStatus,
    pub txid: Option<MeltTx>,
    pub change: Option<Vec<cashu::BlindSignature>>,
}
