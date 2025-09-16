// ----- standard library imports
// ----- extra library imports
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports

// ----- end imports

///--------------------------- Burn tokens
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BurnRequest {
    pub proofs: Vec<cashu::Proof>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BurnResponse {
    pub ys: Vec<cashu::PublicKey>,
}

///--------------------------- Recover tokens
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecoverRequest {
    pub proofs: Vec<cashu::Proof>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecoverResponse {}
