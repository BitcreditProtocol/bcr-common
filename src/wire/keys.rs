// ----- standard library imports
// ----- extra library imports
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports

// ----- end imports

///--------------------------- Generate keyset
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct GenerateKeysetRequest {
    pub qid: uuid::Uuid,
    pub condition: KeysetMintCondition,
    pub expire: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct KeysetMintCondition {
    pub amount: cashu::Amount,
    #[schema(value_type=String)]
    pub public_key: cashu::PublicKey,
}
///--------------------------- Pre-sign blinded message
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct PreSignRequest {
    pub qid: uuid::Uuid,
    pub msg: cashu::BlindedMessage,
}

///--------------------------- Enable keyset
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct EnableKeysetRequest {
    pub qid: uuid::Uuid,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct EnableKeysetResponse {
    pub kid: cashu::Id,
}

///--------------------------- Deactivate keyset
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct DeactivateKeysetRequest {
    pub kid: cashu::Id,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct DeactivateKeysetResponse {
    pub kid: cashu::Id,
}
