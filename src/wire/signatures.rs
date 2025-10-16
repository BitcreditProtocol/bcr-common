// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::{
    core::BillId,
    wire::borsh::{deserialize_chrono_tstamp, serialize_chrono_tstamp},
};

/// --------------------------- request to mint from ebill description
#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct RequestToMintFromEBillDesc {
    pub ebill_id: BillId,
    #[borsh(
        serialize_with = "serialize_chrono_tstamp",
        deserialize_with = "deserialize_chrono_tstamp"
    )]
    pub deadline: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignedRequestToMintFromEBillDesc {
    pub data: RequestToMintFromEBillDesc,
    pub signature: bitcoin::secp256k1::schnorr::Signature,
}

/// --------------------------- request to pay ebill
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequestToMintFromEBillRequest {
    #[schema(value_type = String)]
    pub ebill_id: BillId,
    pub amount: cashu::Amount,
    pub deadline: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequestToMintFromEBillResponse {
    pub request_id: String,
    pub request: String,
}
