// ----- standard library imports
// ----- extra library imports
use cashu::{Amount, CurrencyUnit};
use serde::{Deserialize, Serialize};
// ----- local imports
// ----- end imports

/// Onchain Mint quote request
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct MintQuoteOnchainRequest {
    /// Amount to send and mint
    pub amount: Amount,
    /// Unit wallet would like to receive
    pub unit: CurrencyUnit,
}

/// Onchain Mint quote response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintQuoteOnchainResponse {
    /// Quote ID (UUID v4)
    pub quote: uuid::Uuid,
    /// Bitcoin address to send payment
    pub address: bitcoin::Address<bitcoin::address::NetworkUnchecked>,
    /// Amount received
    pub amount: Amount,
    /// Expiry timestamp
    pub expiry: u64,
}
