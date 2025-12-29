// ----- standard library imports
// ----- extra library imports
use bitcoin::{Address, Amount, address::NetworkUnchecked};
use cashu::CurrencyUnit;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
// ----- end imports

/// Onchain Mint quote request
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct MintQuoteOnchainRequest {
    /// Amount to send and mint
    #[schema(value_type = u64)]
    pub amount: Amount,
    /// Unit wallet would like to receive
    pub unit: CurrencyUnit,
}

/// Onchain Mint quote response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintQuoteOnchainResponse {
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
}
