// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::{
    core::BillId,
    wire::borsh::{deserialize_from_str, deserialize_from_u64, serialize_as_str, serialize_as_u64},
};

/// --------------------------- request to mint from ebill description
#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct RequestToMintFromEBillDesc {
    pub ebill_id: BillId,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub deadline: chrono::DateTime<chrono::Utc>,
    pub sweeping_address: String, // bitcoin::Address is either Serialize or Deserialize
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

/// --------------------------- request to melt
#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct RequestToMeltDesc {
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub qid: uuid::Uuid,
    #[borsh(
        serialize_with = "serialize_as_u64",
        deserialize_with = "deserialize_from_u64"
    )]
    pub amount: cashu::Amount,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignedRequestToMeltDesc {
    pub content: String, // base64 borsh serialized RequestToMeltDesc
    pub signature: bitcoin::secp256k1::schnorr::Signature,
}
