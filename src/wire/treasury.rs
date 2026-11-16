// ----- standard library imports
// ----- extra library imports
use bitcoin::secp256k1;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::{core::BillId, ecash};

// ----- end imports

///--------------------------- Mint operation
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct NewMintOperationRequest {
    pub quote_id: uuid::Uuid,
    #[schema(value_type = String)]
    pub kid: ecash::Id,
    #[schema(value_type = String)]
    pub pub_key: secp256k1::PublicKey,
    /// the ebill face value (bill sum)
    pub amount: u64,
    /// the amount the wallet is allowed to mint, the rest (amount - discounted) is the mint fee
    pub discounted: u64,
    #[schema(value_type = String)]
    pub bill_id: crate::core::BillId,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct NewMintOperationResponse {}

///--------------------------- Mint operation status
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct MintOperationStatus {
    #[schema(value_type = String)]
    pub kid: ecash::Id,
    pub quote_id: uuid::Uuid,
    pub target: u64,
    pub current: u64,
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
    pub proofs: Vec<ecash::Proof>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StoreProofsResponse {}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FeesTokenResponse {
    pub token: String,
    pub total: u64,
    /// `token` carries its own unit; without this the two can silently disagree
    #[serde(default)]
    pub unit: ecash::CurrencyUnit,
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
    /// A foreign mint's unit is exactly what cannot be assumed -- no keyset id reaches here
    #[serde(default)]
    pub unit: ecash::CurrencyUnit,
    pub cr_settled: u64,
    pub cr_unsettled: u64,
    pub db_settled: u64,
    pub db_unsettled: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ForeignBalanceResponse {
    pub balances: Vec<ForeignBalanceEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A foreign mint's balances carry no keyset id, so the unit has to travel with them.
    /// Peers predating the field must still parse, falling back to sat.
    #[test]
    fn foreign_balance_entry_unit_json_wire_compat() {
        let mint_id = crate::core::generate_random_keypair().public_key();
        let entry = ForeignBalanceEntry {
            mint_id,
            unit: ecash::CurrencyUnit::new("eur").expect("unit"),
            cr_settled: 1,
            cr_unsettled: 2,
            db_settled: 3,
            db_unsettled: 4,
        };
        let json = serde_json::to_value(&entry).expect("serialize");
        assert_eq!(json["unit"], "eur");
        assert_eq!(json["cr_settled"], 1);

        let back: ForeignBalanceEntry = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back.unit, entry.unit);

        let legacy = serde_json::json!({
            "mint_id": mint_id.to_string(),
            "cr_settled": 1, "cr_unsettled": 2, "db_settled": 3, "db_unsettled": 4,
        });
        let parsed: ForeignBalanceEntry = serde_json::from_value(legacy).expect("deserialize");
        assert_eq!(parsed.unit, ecash::DEFAULT_CURRENCY_UNIT);
    }

    /// `token` carries its own unit; `total` must agree with it rather than be assumed.
    #[test]
    fn fees_token_response_unit_json_wire_compat() {
        let response = FeesTokenResponse {
            token: String::from("bitcrB..."),
            total: 9,
            unit: ecash::CurrencyUnit::new("usd").expect("unit"),
        };
        let json = serde_json::to_value(&response).expect("serialize");
        assert_eq!(json["unit"], "usd");

        let legacy = serde_json::json!({"token": "bitcrB...", "total": 9});
        let parsed: FeesTokenResponse = serde_json::from_value(legacy).expect("deserialize");
        assert_eq!(parsed.unit, ecash::DEFAULT_CURRENCY_UNIT);
    }
}
