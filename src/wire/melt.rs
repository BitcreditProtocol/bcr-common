// ----- standard library imports
// ----- extra library imports
use bitcoin::{Address, Amount, address::NetworkUnchecked};
use cashu::{CurrencyUnit, MeltQuoteState, nuts::MeltOptions};
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltQuoteOnchainResponse {
    /// Quote ID (UUID v4)
    #[schema(value_type = String)]
    pub quote: uuid::Uuid,
    /// Bitcoin address to send payment
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = String)]
    pub txid: Option<bitcoin::Txid>,
    /// The fee reserve that is required
    #[schema(value_type = u64)]
    pub fee_reserve: Amount,
    /// The amount that needs to be provided
    #[schema(value_type = u64)]
    pub amount: Amount,
    /// Quote State
    pub state: MeltQuoteState,
    /// Unix timestamp until the quote is valid
    pub expiry: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<CurrencyUnit>,
    /// Change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change: Option<Vec<cashu::BlindSignature>>,
}
