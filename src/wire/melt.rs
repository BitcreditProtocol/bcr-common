// ----- standard library imports
// ----- extra library imports
use bitcoin::{Address, Amount, address::NetworkUnchecked};
use cashu::{CurrencyUnit, MeltQuoteState, nuts::MeltOptions};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
// ----- end imports

/// Onchain invoice for melt request
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct OnchainInvoice {
    /// Total BTC amount
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
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltTx {
    #[schema(value_type = Option<String>)]
    pub alpha_txid: Option<bitcoin::Txid>,
    #[schema(value_type = Option<String>)]
    pub beta_txid: Option<bitcoin::Txid>,
}

/// Onchain Melt quote response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltQuoteOnchainResponse {
    /// Quote ID (UUID v4)
    #[schema(value_type = String)]
    pub quote: uuid::Uuid,
    /// Confirmed transaction id after a melt is successful sent onchain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txid: Option<MeltTx>,
    /// The fee reserve that is required
    #[schema(value_type = u64)]
    pub fee_reserve: Amount,
    /// The BTC amount that needs to be provided
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
