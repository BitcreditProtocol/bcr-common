// ----- standard library imports
// ----- extra library imports
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
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
