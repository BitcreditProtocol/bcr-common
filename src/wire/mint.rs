// ----- standard library imports
// ----- extra library imports
use bitcoin::Amount;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::wire::borsh::{
    deserialize_btc_amount, deserialize_from_str, deserialize_vec_of_jsons, serialize_as_str,
    serialize_btc_amount, serialize_vec_of_jsons,
};
// ----- end imports

/// Onchain Mint quote request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnchainMintQuoteRequest {
    /// Blinded messages to be signed upon payment, keyset must be SAT
    pub blinded_messages: Vec<cashu::nuts::BlindedMessage>,
    #[schema(value_type = String)]
    pub wallet_key: cashu::PublicKey,
}

/// Onchain Mint quote response body
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, BorshSerialize, BorshDeserialize)]
pub struct OnchainMintQuoteResponseBody {
    /// Quote ID
    #[schema(value_type = String)]
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub quote: uuid::Uuid,
    /// Bitcoin address to send payment
    pub address: String,
    /// Amount to pay including fees
    #[schema(value_type = u64)]
    #[borsh(
        serialize_with = "serialize_btc_amount",
        deserialize_with = "deserialize_btc_amount"
    )]
    pub payment_amount: Amount,
    /// Quote expiry timestamp
    pub expiry: u64,
    /// Blinded messages committed to
    #[borsh(
        serialize_with = "serialize_vec_of_jsons",
        deserialize_with = "deserialize_vec_of_jsons"
    )]
    pub blinded_messages: Vec<cashu::nuts::BlindedMessage>,
    #[schema(value_type = String)]
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub wallet_key: cashu::PublicKey,
}

/// Onchain Mint Request to Fetch Signatures
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnchainMintRequest {
    /// Quote ID
    #[schema(value_type = String)]
    pub quote: uuid::Uuid,
    /// Id of the origin mint
    #[schema(value_type = String)]
    pub alpha_id: bitcoin::secp256k1::PublicKey,
}

/// Onchain Mint quote response with commitment signature
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnchainMintQuoteResponse {
    pub content: String, // base64, borsh serialized OnchainMintQuoteResponseBody
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

/// Mint response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintResponse {
    pub signatures: Vec<cashu::BlindSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintProtestRequest {
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

pub use crate::wire::common::ProtestStatus;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintProtestResponse {
    pub status: ProtestStatus,
    pub signatures: Option<Vec<cashu::nuts::BlindSignature>>,
}
