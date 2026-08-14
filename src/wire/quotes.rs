// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
// ----- local imports
use crate::{
    core::{BillId, NodeId},
    wire::{
        bill::{BillIdentParticipant, BillParticipant},
        borsh::{
            deserialize_bill_date, deserialize_from_str, deserialize_vec_of_strs, serialize_as_str,
            serialize_bill_date, serialize_vec_of_strs,
        },
    },
};

// ----- end imports

///--------------------------- Enquire mint quote
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct SharedBill {
    #[schema(value_type = String)]
    pub bill_id: BillId,
    pub data: String, // The base58 encoded, encrypted, borshed BillBlockPlaintextWrappers of the bill
    #[borsh(
        serialize_with = "serialize_vec_of_strs",
        deserialize_with = "deserialize_vec_of_strs"
    )]
    #[schema(value_type = Vec<String>)]
    pub file_urls: Vec<url::Url>,
    pub hash: String,
    pub signature: String,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    #[schema(value_type = String)]
    pub receiver: bitcoin::PublicKey,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize, ToSchema)]
pub struct BillInfo {
    #[schema(value_type = String)]
    pub id: BillId,
    pub drawee: BillIdentParticipant,
    pub drawer: BillIdentParticipant,
    pub payee: BillParticipant,
    pub endorsees: Vec<BillParticipant>,
    pub sum: u64, // in satoshis, converted to bitcoin::Amount in the service
    #[borsh(
        serialize_with = "serialize_bill_date",
        deserialize_with = "deserialize_bill_date"
    )]
    #[serde(with = "crate::wire::bill_date")]
    pub maturity_date: time::Date,
    #[borsh(
        serialize_with = "serialize_vec_of_strs",
        deserialize_with = "deserialize_vec_of_strs"
    )]
    #[schema(value_type = Vec<String>)]
    pub file_urls: Vec<url::Url>, // urls of files, encrypted and uploaded for the mint to the mint's relay
}

///--------------------------- Enquire mint quote
#[derive(Debug, ToSchema, BorshSerialize, BorshDeserialize)]
pub struct EnquireRequest {
    pub content: SharedBill,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    /// corresponding secret key must be used later in key_client::mint request
    pub minting_pubkey: cashu::PublicKey,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SignedEnquireRequest {
    pub content: String, // base64, borsh serialized EnquireRequest
    #[schema(value_type = String)]
    pub signature: bitcoin::secp256k1::schnorr::Signature,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EnquireReply {
    pub id: uuid::Uuid,
}

/// --------------------------- Look up quote
/// StatusReply for quote status look up by users
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "status")]
pub enum StatusReply {
    Pending,
    Canceled {
        #[serde(with = "time::serde::rfc3339")]
        tstamp: time::OffsetDateTime,
    },
    Denied {
        #[serde(with = "time::serde::rfc3339")]
        tstamp: time::OffsetDateTime,
    },
    Offered {
        keyset_id: cashu::Id,
        #[serde(with = "time::serde::rfc3339")]
        expiration_date: time::OffsetDateTime,
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
        wallet_pubkey: cashu::PublicKey,
    },
    OfferExpired {
        #[serde(with = "time::serde::rfc3339")]
        tstamp: time::OffsetDateTime,
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
    },
    Accepted {
        keyset_id: cashu::Id,
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
        wallet_pubkey: cashu::PublicKey,
    },
    Rejected {
        #[serde(with = "time::serde::rfc3339")]
        tstamp: time::OffsetDateTime,
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
    },
    MintingEnabled {
        keyset_id: cashu::Id,
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
        wallet_pubkey: cashu::PublicKey,
        minted_amount: cashu::Amount,
    },
    FailedEbillValidation {
        keyset_id: cashu::Id,
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
        wallet_pubkey: cashu::PublicKey,
    },
}

/// --------------------------- List quotes
#[derive(Debug, Default, Serialize, Deserialize, IntoParams)]
pub struct ListParam {
    #[serde(default, with = "crate::wire::bill_date::option")]
    pub bill_maturity_date_from: Option<time::Date>,
    #[serde(default, with = "crate::wire::bill_date::option")]
    pub bill_maturity_date_to: Option<time::Date>,
    pub status: Option<InfoReplyDiscriminants>,
    #[param(value_type = Option<String>)]
    pub bill_id: Option<BillId>,
    #[param(value_type = Option<String>)]
    pub bill_drawee_id: Option<NodeId>,
    #[param(value_type = Option<String>)]
    pub bill_drawer_id: Option<NodeId>,
    #[param(value_type = Option<String>)]
    pub bill_payer_id: Option<NodeId>,
    #[param(value_type = Option<String>)]
    pub bill_holder_id: Option<NodeId>,
    pub sort: Option<ListSort>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ListSort {
    BillMaturityDateDesc,
    BillMaturityDateAsc,
    SubmittedDesc,
    SubmittedAsc,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListReply {
    pub quotes: Vec<uuid::Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LightInfo {
    pub id: uuid::Uuid,
    pub status: InfoReplyDiscriminants,
    #[schema(value_type = u64)]
    pub sum: bitcoin::Amount,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListReplyLight {
    pub quotes: Vec<LightInfo>,
}

pub use super::common::{PaginatedResponse, Pagination};

/// --------------------------- Quote info request
#[derive(Debug, Serialize, Deserialize, ToSchema, strum::EnumDiscriminants)]
#[serde(rename_all = "PascalCase", tag = "status")]
#[strum_discriminants(derive(Serialize, Deserialize, ToSchema, strum::Display))]
pub enum InfoReply {
    Pending {
        id: uuid::Uuid,
        bill: BillInfo,
        #[serde(with = "time::serde::rfc3339")]
        submitted: time::OffsetDateTime,
        #[serde(with = "time::serde::rfc3339")]
        suggested_expiration: time::OffsetDateTime,
    },
    Canceled {
        id: uuid::Uuid,
        bill: BillInfo,
        #[serde(with = "time::serde::rfc3339")]
        tstamp: time::OffsetDateTime,
    },
    Offered {
        id: uuid::Uuid,
        bill: BillInfo,
        #[serde(with = "time::serde::rfc3339")]
        ttl: time::OffsetDateTime,
        keyset_id: cashu::Id,
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
    },
    OfferExpired {
        id: uuid::Uuid,
        bill: BillInfo,
        #[serde(with = "time::serde::rfc3339")]
        tstamp: time::OffsetDateTime,
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
    },
    Denied {
        id: uuid::Uuid,
        bill: BillInfo,
        #[serde(with = "time::serde::rfc3339")]
        tstamp: time::OffsetDateTime,
    },
    Accepted {
        id: uuid::Uuid,
        bill: BillInfo,
        keyset_id: cashu::Id,
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
    },
    Rejected {
        id: uuid::Uuid,
        bill: BillInfo,
        #[serde(with = "time::serde::rfc3339")]
        tstamp: time::OffsetDateTime,
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
    },
    MintingEnabled {
        id: uuid::Uuid,
        bill: BillInfo,
        keyset_id: cashu::Id,
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
        fee: cashu::Amount,
    },
    FailedEbillValidation {
        id: uuid::Uuid,
        bill: BillInfo,
        keyset_id: cashu::Id,
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
    },
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListPendingQueryRequest {
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub since: Option<time::OffsetDateTime>,
}

/// --------------------------- Update quote status request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase", tag = "action")]
pub enum UpdateQuoteRequest {
    Deny,
    Offer {
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
        #[serde(default, with = "time::serde::rfc3339::option")]
        ttl: Option<time::OffsetDateTime>,
    },
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase", tag = "status")]
pub enum UpdateQuoteResponse {
    Denied,
    Offered {
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
        #[serde(with = "time::serde::rfc3339")]
        ttl: time::OffsetDateTime,
    },
}

/// --------------------------- Resolve quote
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase", tag = "action")]
pub enum ResolveOffer {
    Reject,
    Accept,
}

/// --------------------------- Get encrypted bill file from request to mint
#[derive(Debug, Serialize, Deserialize, IntoParams, ToSchema)]
pub struct RequestEncryptedFileUrlPayload {
    #[schema(value_type = String)]
    pub file_url: url::Url,
}

/// --------------------------- Enable minting of accepted quote
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EnableMintingRequest {}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EnableMintingResponse {}

///--------------------------- Fetch Data based on a shared bill
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct SharedBillData {
    #[schema(value_type = String)]
    pub bill_id: BillId,
    pub data: String, // The base58 encoded, encrypted, borshed BillBlockPlaintextWrappers of the bill
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tstamp_json_wire_compat() {
        let reply = StatusReply::Canceled {
            tstamp: time::macros::datetime!(2026-08-03 12:00:00 UTC),
        };
        let json = serde_json::to_string(&reply).unwrap();
        assert_eq!(
            json,
            r#"{"status":"Canceled","tstamp":"2026-08-03T12:00:00Z"}"#
        );
        let deserialized: StatusReply = serde_json::from_str(&json).unwrap();
        assert!(
            matches!(deserialized, StatusReply::Canceled { tstamp } if tstamp.unix_timestamp() == 1_785_758_400)
        );
    }

    #[test]
    fn option_tstamp_json_wire_compat() {
        let request = UpdateQuoteRequest::Offer {
            discounted: bitcoin::Amount::from_sat(7),
            ttl: Some(time::macros::datetime!(2026-08-03 12:00:00 UTC)),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"action":"Offer","discounted":7,"ttl":"2026-08-03T12:00:00Z"}"#
        );
        let request = UpdateQuoteRequest::Offer {
            discounted: bitcoin::Amount::from_sat(7),
            ttl: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"action":"Offer","discounted":7,"ttl":null}"#);
        let missing: UpdateQuoteRequest =
            serde_json::from_str(r#"{"action":"Offer","discounted":7}"#).unwrap();
        assert!(matches!(
            missing,
            UpdateQuoteRequest::Offer { ttl: None, .. }
        ));
    }

    #[test]
    fn option_bill_date_json_wire_compat() {
        let params = ListParam {
            bill_maturity_date_from: Some(time::macros::date!(2026 - 08 - 03)),
            ..Default::default()
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["bill_maturity_date_from"], "2026-08-03");
        assert_eq!(json["bill_maturity_date_to"], serde_json::Value::Null);
        let deserialized: ListParam = serde_json::from_value(json).unwrap();
        assert_eq!(
            deserialized.bill_maturity_date_from,
            Some(time::macros::date!(2026 - 08 - 03))
        );
        let missing: ListParam = serde_json::from_str("{}").unwrap();
        assert_eq!(missing.bill_maturity_date_from, None);
    }
}
