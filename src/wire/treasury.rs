// ----- standard library imports
// ----- extra library imports
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::core::BillId;
// ----- local imports

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
    pub deadline: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequestToPayFromEBillResponse {}
