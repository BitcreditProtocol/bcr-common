// ----- standard library imports
// ----- extra library imports
use bitcoin::secp256k1;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::core::BillId;

// ----- end imports

///--------------------------- Mint operation
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct NewMintOperationRequest {
    pub quote_id: uuid::Uuid,
    pub kid: cashu::Id,
    pub pub_key: cashu::PublicKey,
    pub target: cashu::Amount,
    #[schema(value_type = String)]
    pub bill_id: crate::core::BillId,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct NewMintOperationResponse {}

///--------------------------- Mint operation status
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct MintOperationStatus {
    pub kid: cashu::Id,
    pub quote_id: uuid::Uuid,
    pub target: cashu::Amount,
    pub current: cashu::Amount,
}

/// --------------------------- request to pay ebill
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequestToPayFromEBillRequest {
    #[schema(value_type = String)]
    pub ebill_id: BillId,
    #[schema(value_type = u64)]
    pub amount: bitcoin::Amount,
    #[serde(with = "time::serde::rfc3339")]
    pub deadline: time::OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequestToPayFromEBillResponse {}

/// --------------------------- collecting fees
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StoreProofsRequest {
    pub proofs: Vec<cashu::Proof>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StoreProofsResponse {}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FeesTokenResponse {
    pub token: String,
    pub total: cashu::Amount,
}

///--------------------------- denied melt operations
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeniedMeltOp {
    pub id: uuid::Uuid,
    #[schema(value_type = u64)]
    pub amount: bitcoin::Amount,
    #[serde(with = "time::serde::rfc3339")]
    pub created: time::OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeniedMeltOperations {
    pub ops: Vec<DeniedMeltOp>,
}

///--------------------------- foreign eCash balance
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ForeignBalanceEntry {
    #[schema(value_type = String)]
    pub mint_id: secp256k1::PublicKey,
    pub settled: cashu::Amount,
    pub unsettled: cashu::Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ForeignBalanceResponse {
    pub balances: Vec<ForeignBalanceEntry>,
}
