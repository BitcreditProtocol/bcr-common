// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::{
    core::BillId,
    wire::borsh::{deserialize_as_str, serialize_as_str},
};

/// --------------------------- request to mint from ebill description
#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct RequestToMintFromEBillDesc {
    pub ebill_id: BillId,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_as_str"
    )]
    pub deadline: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignedRequestToMintFromEBillDesc {
    pub content: String, // base64 borsh serialized RequestToMintFromEBillDesc
    pub signature: bitcoin::secp256k1::schnorr::Signature,
}

/// --------------------------- request to pay ebill
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequestToMintFromEBillRequest {
    #[schema(value_type = String)]
    pub ebill_id: BillId,
    #[schema(value_type = u64)]
    pub amount: bitcoin::Amount,
    pub deadline: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequestToMintFromEBillResponse {
    pub request_id: String,
    pub request: String,
}
