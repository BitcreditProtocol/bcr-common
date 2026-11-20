// ----- standard library imports
// ----- extra library imports
use bitcoin::secp256k1 as secp;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
// ----- local imports
use crate::{
    core::{BillId, NodeId},
    ecash,
    wire::{
        bill::{BillIdentParticipant, BillParticipant},
        borsh::{
            deserialize_bill_date, deserialize_from_str, deserialize_vec_of_strs, serialize_as_str,
            serialize_bill_date, serialize_vec_of_strs,
        },
        common::secp_compat,
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
    /// In satoshis. This is part of a signed borsh payload (`SignedEnquireRequest`), so it
    /// cannot gain a unit field without invalidating existing signatures -- a versioned
    /// successor is needed before a bill here can be denominated in anything else.
    pub sum: u64,
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
    #[schema(value_type = String)]
    pub minting_pubkey: secp::PublicKey,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SignedEnquireRequest {
    pub content: String, // base64, borsh serialized EnquireRequest
    #[schema(value_type = String)]
    pub signature: secp::schnorr::Signature,
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
        discounted: u64,
        #[schema(value_type = String)]
        #[serde(with = "secp_compat")]
        wallet_pubkey: secp::PublicKey,
    },
    OfferExpired {
        #[serde(with = "time::serde::rfc3339")]
        tstamp: time::OffsetDateTime,
        discounted: u64,
        /// Terminal status: no `keyset_id` reaches here, so the currency of `discounted`
        /// would otherwise be lost
        #[serde(default)]
        unit: ecash::CurrencyUnit,
    },
    Accepted {
        keyset_id: cashu::Id,
        discounted: u64,
        #[schema(value_type = String)]
        #[serde(with = "secp_compat")]
        wallet_pubkey: secp::PublicKey,
    },
    Rejected {
        #[serde(with = "time::serde::rfc3339")]
        tstamp: time::OffsetDateTime,
        discounted: u64,
        /// Terminal status: no `keyset_id` reaches here, so the currency of `discounted`
        /// would otherwise be lost
        #[serde(default)]
        unit: ecash::CurrencyUnit,
    },
    MintingEnabled {
        keyset_id: cashu::Id,
        discounted: u64,
        #[schema(value_type = String)]
        #[serde(with = "secp_compat")]
        wallet_pubkey: secp::PublicKey,
        minted_amount: u64,
    },
    FailedEbillValidation {
        keyset_id: cashu::Id,
        discounted: u64,
        #[schema(value_type = String)]
        #[serde(with = "secp_compat")]
        wallet_pubkey: secp::PublicKey,
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
    AmountDesc,
    AmountAsc,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListReply {
    pub quotes: Vec<uuid::Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LightInfo {
    pub id: uuid::Uuid,
    pub status: InfoReplyDiscriminants,
    pub sum: u64,
    /// A list entry carries no keyset id, so the sum needs its own unit
    #[serde(default)]
    pub unit: ecash::CurrencyUnit,
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
        discounted: u64,
    },
    OfferExpired {
        id: uuid::Uuid,
        bill: BillInfo,
        #[serde(with = "time::serde::rfc3339")]
        tstamp: time::OffsetDateTime,
        discounted: u64,
        /// Terminal status: no `keyset_id` reaches here, so the currency of `discounted`
        /// would otherwise be lost
        #[serde(default)]
        unit: ecash::CurrencyUnit,
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
        discounted: u64,
    },
    Rejected {
        id: uuid::Uuid,
        bill: BillInfo,
        #[serde(with = "time::serde::rfc3339")]
        tstamp: time::OffsetDateTime,
        discounted: u64,
        /// Terminal status: no `keyset_id` reaches here, so the currency of `discounted`
        /// would otherwise be lost
        #[serde(default)]
        unit: ecash::CurrencyUnit,
    },
    MintingEnabled {
        id: uuid::Uuid,
        bill: BillInfo,
        keyset_id: cashu::Id,
        discounted: u64,
        fee: u64,
    },
    FailedEbillValidation {
        id: uuid::Uuid,
        bill: BillInfo,
        keyset_id: cashu::Id,
        discounted: u64,
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
        discounted: u64,
        /// standalone request: no keyset id sibling to resolve the unit from
        #[serde(default)]
        unit: ecash::CurrencyUnit,
        #[serde(default, with = "time::serde::rfc3339::option")]
        ttl: Option<time::OffsetDateTime>,
    },
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase", tag = "status")]
pub enum UpdateQuoteResponse {
    Denied,
    Offered {
        discounted: u64,
        /// standalone response: no keyset id sibling to resolve the unit from
        #[serde(default)]
        unit: ecash::CurrencyUnit,
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

    fn sample_bill_info() -> BillInfo {
        let (_, participant) = crate::wire::test_utils::random_identity_public_data();
        BillInfo {
            id: crate::core::test_utils::random_bill_id(),
            drawee: participant.clone(),
            drawer: participant.clone(),
            payee: BillParticipant::Ident(participant.clone()),
            endorsees: vec![],
            sum: 1500,
            maturity_date: time::macros::date!(2026 - 11 - 03),
            file_urls: vec![],
        }
    }

    /// `OfferExpired` and `Rejected` are the only `discounted`-bearing variants with no
    /// `keyset_id` to resolve the unit from, so they carry it themselves. A client that
    /// fetches a terminal quote first must still be able to read the amount.
    #[test]
    fn terminal_quote_statuses_keep_their_currency() {
        let tstamp = time::macros::datetime!(2026-08-03 12:00:00 UTC);
        let eur = ecash::CurrencyUnit::new("eur").expect("unit");

        let expired = StatusReply::OfferExpired {
            tstamp,
            discounted: 1200,
            unit: eur,
        };
        let json = serde_json::to_value(&expired).expect("serialize");
        assert_eq!(json["status"], "OfferExpired");
        assert_eq!(json["discounted"], 1200);
        assert_eq!(json["unit"], "eur");

        let rejected = StatusReply::Rejected {
            tstamp,
            discounted: 1200,
            unit: eur,
        };
        assert_eq!(
            serde_json::to_value(&rejected).expect("serialize")["unit"],
            "eur"
        );

        // a terminal status written before the field still parses, meaning sat
        for status in ["OfferExpired", "Rejected"] {
            let legacy = serde_json::json!({
                "status": status,
                "tstamp": "2026-08-03T12:00:00Z",
                "discounted": 1200,
            });
            let parsed: StatusReply = serde_json::from_value(legacy).expect("deserialize");
            let unit = match parsed {
                StatusReply::OfferExpired { unit, .. } | StatusReply::Rejected { unit, .. } => unit,
                other => panic!("expected a terminal status, got {other:?}"),
            };
            assert_eq!(unit, ecash::DEFAULT_CURRENCY_UNIT, "{status}");
        }

        // the same two variants of the admin-facing reply
        let legacy = serde_json::json!({
            "status": "OfferExpired",
            "id": uuid::Uuid::nil(),
            "bill": serde_json::to_value(sample_bill_info()).expect("bill"),
            "tstamp": "2026-08-03T12:00:00Z",
            "discounted": 1200,
        });
        let parsed: InfoReply = serde_json::from_value(legacy).expect("deserialize");
        assert!(
            matches!(parsed, InfoReply::OfferExpired { unit, .. } if unit == ecash::DEFAULT_CURRENCY_UNIT)
        );
    }

    #[test]
    fn option_tstamp_json_wire_compat() {
        let request = UpdateQuoteRequest::Offer {
            discounted: 7,
            unit: ecash::DEFAULT_CURRENCY_UNIT,
            ttl: Some(time::macros::datetime!(2026-08-03 12:00:00 UTC)),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"action":"Offer","discounted":7,"unit":"sat","ttl":"2026-08-03T12:00:00Z"}"#
        );
        let request = UpdateQuoteRequest::Offer {
            discounted: 7,
            unit: ecash::DEFAULT_CURRENCY_UNIT,
            ttl: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"action":"Offer","discounted":7,"unit":"sat","ttl":null}"#
        );
        // a payload predating both optional fields still parses, unit falling back to sat
        let missing: UpdateQuoteRequest =
            serde_json::from_str(r#"{"action":"Offer","discounted":7}"#).unwrap();
        assert!(matches!(
            missing,
            UpdateQuoteRequest::Offer { ttl: None, unit, .. } if unit == ecash::DEFAULT_CURRENCY_UNIT
        ));

        // a non-sat offer survives the round trip
        let eur = UpdateQuoteRequest::Offer {
            discounted: 7,
            unit: ecash::CurrencyUnit::new("eur").expect("unit"),
            ttl: None,
        };
        let json = serde_json::to_string(&eur).unwrap();
        assert_eq!(
            json,
            r#"{"action":"Offer","discounted":7,"unit":"eur","ttl":null}"#
        );
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

    /// The quote messages as they were laid out before their key fields became
    /// `secp::PublicKey`.
    mod wire_compat {
        use super::*;
        use crate::{cashu, core, core_tests, ecash, wire::common::assert_wire_compat};

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        #[serde(tag = "status")]
        enum StatusReplyV1 {
            Offered {
                keyset_id: cashu::Id,
                #[serde(with = "time::serde::rfc3339")]
                expiration_date: time::OffsetDateTime,
                discounted: u64,
                wallet_pubkey: cashu::PublicKey,
            },
            Accepted {
                keyset_id: cashu::Id,
                discounted: u64,
                wallet_pubkey: cashu::PublicKey,
            },
            MintingEnabled {
                keyset_id: cashu::Id,
                discounted: u64,
                wallet_pubkey: cashu::PublicKey,
                minted_amount: u64,
            },
            FailedEbillValidation {
                keyset_id: cashu::Id,
                discounted: u64,
                wallet_pubkey: cashu::PublicKey,
            },
        }

        #[derive(BorshSerialize, BorshDeserialize)]
        struct EnquireRequestV1 {
            content: SharedBill,
            #[borsh(
                serialize_with = "serialize_as_str",
                deserialize_with = "deserialize_from_str"
            )]
            minting_pubkey: cashu::PublicKey,
        }

        fn keyset_id() -> cashu::Id {
            cashu::Id::from_bytes(&[0, 173, 38, 140, 77, 31, 88, 38]).expect("id")
        }

        fn key() -> secp::PublicKey {
            core::generate_random_keypair().public_key()
        }

        fn as_cashu(key: &secp::PublicKey) -> cashu::PublicKey {
            ecash::public_key_secp2cashu(key)
        }

        #[test]
        fn status_reply_offered_wire_compat() {
            let wallet_pubkey = key();
            let expiration_date = time::macros::datetime!(2026-08-03 12:00:00 UTC);
            assert_wire_compat(
                &StatusReply::Offered {
                    keyset_id: keyset_id(),
                    expiration_date,
                    discounted: 1200,
                    wallet_pubkey,
                },
                &StatusReplyV1::Offered {
                    keyset_id: keyset_id(),
                    expiration_date,
                    discounted: 1200,
                    wallet_pubkey: as_cashu(&wallet_pubkey),
                },
            );
        }

        #[test]
        fn status_reply_accepted_wire_compat() {
            let wallet_pubkey = key();
            assert_wire_compat(
                &StatusReply::Accepted {
                    keyset_id: keyset_id(),
                    discounted: 1200,
                    wallet_pubkey,
                },
                &StatusReplyV1::Accepted {
                    keyset_id: keyset_id(),
                    discounted: 1200,
                    wallet_pubkey: as_cashu(&wallet_pubkey),
                },
            );
        }

        #[test]
        fn status_reply_minting_enabled_wire_compat() {
            let wallet_pubkey = key();
            assert_wire_compat(
                &StatusReply::MintingEnabled {
                    keyset_id: keyset_id(),
                    discounted: 1200,
                    wallet_pubkey,
                    minted_amount: 1000,
                },
                &StatusReplyV1::MintingEnabled {
                    keyset_id: keyset_id(),
                    discounted: 1200,
                    wallet_pubkey: as_cashu(&wallet_pubkey),
                    minted_amount: 1000,
                },
            );
        }

        #[test]
        fn status_reply_failed_ebill_validation_wire_compat() {
            let wallet_pubkey = key();
            assert_wire_compat(
                &StatusReply::FailedEbillValidation {
                    keyset_id: keyset_id(),
                    discounted: 1200,
                    wallet_pubkey,
                },
                &StatusReplyV1::FailedEbillValidation {
                    keyset_id: keyset_id(),
                    discounted: 1200,
                    wallet_pubkey: as_cashu(&wallet_pubkey),
                },
            );
        }

        /// `EnquireRequest` is borshed and then schnorr-signed, so a diverging byte here
        /// would invalidate every signature already issued over one.
        #[test]
        fn enquire_request_borsh_wire_compat() {
            let minting_pubkey = key();
            let content = SharedBill {
                bill_id: core_tests::random_bill_id(),
                data: "payload".to_string(),
                file_urls: vec![],
                hash: "a-hash".to_string(),
                signature: "a-signature".to_string(),
                receiver: bitcoin::PublicKey::new(key()),
            };
            let request = EnquireRequest {
                content: content.clone(),
                minting_pubkey,
            };
            let v1 = EnquireRequestV1 {
                content,
                minting_pubkey: as_cashu(&minting_pubkey),
            };
            let bytes = borsh::to_vec(&request).expect("serialize");
            let v1_bytes = borsh::to_vec(&v1).expect("serialize");
            assert_eq!(bytes, v1_bytes, "borsh encodings diverge");
            let back: EnquireRequest =
                borsh::from_slice(&v1_bytes).expect("the new type reads a v1 blob");
            assert_eq!(borsh::to_vec(&back).expect("serialize"), bytes);
            assert_eq!(back.minting_pubkey, minting_pubkey);
        }
    }
}
