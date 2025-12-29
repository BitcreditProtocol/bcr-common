// ----- standard library imports
// ----- extra library imports
use bitcoin::{Address, Amount, address::NetworkUnchecked};
use cashu::{CurrencyUnit, nuts::MeltOptions};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
// ----- end imports

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct OnchainInvoice {
    /// Total amount
    #[schema(value_type = u64)]
    pub amount: Amount,
    /// Bitcoin address to pay
    #[schema(value_type = String)]
    pub address: Address<NetworkUnchecked>,
}

/// Onchain Melt quote request
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct MeltQuoteOnchainRequest {
    /// Bitcoin Address
    pub request: OnchainInvoice,
    /// Unit wallet would like to pay with
    pub unit: CurrencyUnit,
    /// Payment Options
    pub options: Option<MeltOptions>,
    /// Wallet signature
    #[schema(value_type = String)]
    pub signature: bitcoin::secp256k1::schnorr::Signature,
}
