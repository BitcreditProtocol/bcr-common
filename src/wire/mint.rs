// ----- standard library imports
// ----- extra library imports
use bitcoin::{Address, Amount, address::NetworkUnchecked};
use cashu::CurrencyUnit;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintQuoteOnchainResponseBody {
    /// Quote ID (UUID v4)
    #[schema(value_type = String)]
    pub quote: uuid::Uuid,
    /// Bitcoin address to send payment
    #[schema(value_type = String)]
    pub address: Address<NetworkUnchecked>,
    /// Amount received
    #[schema(value_type = u64)]
    pub amount: Amount,
    /// Expiry timestamp
    pub expiry: u64,
    /// Blinded messages committed to
    pub blinded_messages: Vec<cashu::nuts::BlindedMessage>,
}

/// Onchain Mint quote response with commitment signature
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintQuoteOnchainResponse {
    pub body: MintQuoteOnchainResponseBody,
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
    pub state: Option<cashu::MintQuoteState>,
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
