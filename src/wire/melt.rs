// ----- standard library imports
// ----- extra library imports
use cashu::{Amount, CurrencyUnit, nuts::MeltOptions};
use serde::{Deserialize, Serialize};
// ----- local imports
// ----- end imports

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnChainRequest {
    /// Total amount
    pub amount: Amount,
    /// Bitcoin address to pay
    pub address: bitcoin::Address<bitcoin::address::NetworkUnchecked>,
}

/// Onchain Melt quote request
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeltQuoteOnchainRequest {
    /// Bitcoin Address
    pub request: OnChainRequest,
    /// Unit wallet would like to pay with
    pub unit: CurrencyUnit,
    /// Payment Options
    pub options: Option<MeltOptions>,
    /// Wallet signature
    pub signature: bitcoin::secp256k1::schnorr::Signature,
}
