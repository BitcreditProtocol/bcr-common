// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::core::BillId;

/// --------------------------- request to mint from ebill description
#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct RequestToMintFromEBillDesc {
    pub ebill_id: BillId,
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
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequestToMintFromEBillResponse {
    pub request_id: String,
    pub request: String,
}
