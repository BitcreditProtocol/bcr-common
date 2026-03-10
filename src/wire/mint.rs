// ----- standard library imports
// ----- extra library imports
use bitcoin::{Address, Amount, address::NetworkUnchecked};
use borsh::{BorshDeserialize, BorshSerialize};
use cashu::CurrencyUnit;
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
pub struct MintQuoteOnchainRequest {
    /// Amount to send and mint
    #[schema(value_type = u64)]
    pub amount: Amount,
    /// Unit wallet would like to receive
    pub unit: CurrencyUnit,
    /// Blinded messages to be signed upon payment
    pub blinded_messages: Vec<cashu::nuts::BlindedMessage>,
}

/// Onchain Mint quote response body
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, BorshSerialize, BorshDeserialize)]
pub struct MintQuoteOnchainResponseBody {
    /// Quote ID (UUID v4)
    #[schema(value_type = String)]
    #[borsh(serialize_with = "serialize_as_str", deserialize_with = "deserialize_from_str")]
    pub quote: uuid::Uuid,
    /// Bitcoin address to send payment
    #[schema(value_type = String)]
    #[borsh(serialize_with = "serialize_as_str", deserialize_with = "deserialize_from_str")]
    pub address: Address<NetworkUnchecked>,
    /// Amount received
    #[schema(value_type = u64)]
    #[borsh(serialize_with = "serialize_btc_amount", deserialize_with = "deserialize_btc_amount")]
    pub amount: Amount,
    /// Expiry timestamp
    pub expiry: u64,
    /// Blinded messages committed to
    #[borsh(serialize_with = "serialize_vec_of_jsons", deserialize_with = "deserialize_vec_of_jsons")]
    pub blinded_messages: Vec<cashu::nuts::BlindedMessage>,
}

/// Onchain Mint quote response with commitment signature
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintQuoteOnchainResponse {
    pub content: String, // base64, borsh serialized MintQuoteOnchainResponseBody
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintProtestRequest {
    #[schema(value_type = String)]
    pub alpha_id: bitcoin::secp256k1::PublicKey,
    #[schema(value_type = String)]
    pub quote_id: uuid::Uuid,
    pub body: MintQuoteOnchainResponseBody,
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
    pub payment_height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintOnchainTrigger {
    #[schema(value_type = String)]
    pub quote_id: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum ProtestStatus {
    Resolved,
    Rabid,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintProtestResponse {
    pub status: ProtestStatus,
    pub signatures: Option<Vec<cashu::nuts::BlindSignature>>,
}
