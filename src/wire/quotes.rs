// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
// ----- local imports
use crate::{
    core::{BillId, NodeId},
    wire::{
        bill::{BillIdentParticipant, BillParticipant},
        borsh::{
            deserialize_from_str, deserialize_vec_of_strs, serialize_as_str, serialize_vec_of_strs,
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
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub maturity_date: chrono::NaiveDate,
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

pub const REISSUE_ENQUIRE_SCHEMA_VERSION: &str = "credit-quote-reissue-enquiry-v1";
pub const REISSUE_ENQUIRE_ACTION: &str = "reissue_denied_quote_after_reviewed_correction";

/// The independently signed AI Credit authority for one corrected, terminal quote.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, BorshSerialize, BorshDeserialize,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreditQuoteReissuePermit {
    pub schema_version: String,
    pub key_id: String,
    pub mint_id: String,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub previous_mint_quote_id: uuid::Uuid,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub reissued_mint_quote_id: uuid::Uuid,
    pub credit_program_version: String,
    pub credit_program_digest: String,
    pub case_id: String,
    pub bill_id: String,
    pub bill_state_digest: String,
    pub holder_ref: String,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub review_request_id: uuid::Uuid,
    pub contested_decision_result_digest: String,
    pub corrected_submission_digest: String,
    pub issued_at: String,
    pub expires_at: String,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub nonce: uuid::Uuid,
    pub action: String,
    pub synthetic: bool,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, BorshSerialize, BorshDeserialize,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SignedCreditQuoteReissuePermit {
    pub permit: CreditQuoteReissuePermit,
    pub permit_digest: String,
    pub signature_algorithm: String,
    pub signature: String,
}

/// Holder consent for exactly one AI-authorized replacement of a terminal quote.
#[derive(Debug, ToSchema, BorshSerialize, BorshDeserialize)]
pub struct ReissueEnquireRequestV1 {
    pub schema_version: String,
    pub action: String,
    pub content: SharedBill,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub minting_pubkey: cashu::PublicKey,
    pub signed_permit: SignedCreditQuoteReissuePermit,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SignedReissueEnquireRequestV1 {
    pub content: String, // base64, borsh serialized ReissueEnquireRequestV1
    #[schema(value_type = String)]
    pub signature: bitcoin::secp256k1::schnorr::Signature,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EnquireReply {
    pub id: uuid::Uuid,
}

/// --------------------------- Look up quote
/// StatusReply for quote status look up by users
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "status")]
pub enum StatusReply {
    Pending,
    Canceled {
        tstamp: DateTime<Utc>,
    },
    Denied {
        tstamp: DateTime<Utc>,
    },
    Offered {
        keyset_id: cashu::Id,
        expiration_date: DateTime<Utc>,
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
        wallet_pubkey: cashu::PublicKey,
    },
    OfferExpired {
        tstamp: DateTime<Utc>,
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
        tstamp: DateTime<Utc>,
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

/// Opaque pointer telling the applicant-facing client that governed input is required.
///
/// The referenced questions and any access credentials deliberately live outside the public
/// quote API. The digest lets that client reconcile the exact revision through its separately
/// authenticated applicant channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplicantActionProjection {
    pub kind: ApplicantActionKind,
    pub revision_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApplicantActionKind {
    Clarification,
}

/// Public quote lookup response. `quote` remains the existing lifecycle state machine; the
/// optional action is an independent projection and therefore cannot introduce a quote status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuoteStatusReply {
    #[serde(flatten)]
    pub quote: StatusReply,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applicant_action: Option<ApplicantActionProjection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CreditApplicantAction {
    ClarificationRequired,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreditApplicantActionReceipt {
    pub schema_version: String,
    pub operation_id: String,
    pub mint_quote_id: uuid::Uuid,
    pub credit_program_version: String,
    pub credit_program_digest: String,
    pub revision_digest: String,
    pub expected_revision_digest: Option<String>,
    pub applicant_action: CreditApplicantAction,
    pub action: String,
    pub status: String,
    pub completed_at: String,
}

/// --------------------------- List quotes
#[derive(Debug, Default, Serialize, Deserialize, IntoParams)]
pub struct ListParam {
    pub bill_maturity_date_from: Option<chrono::NaiveDate>,
    pub bill_maturity_date_to: Option<chrono::NaiveDate>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
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
        submitted: DateTime<Utc>,
        suggested_expiration: DateTime<Utc>,
    },
    Canceled {
        id: uuid::Uuid,
        bill: BillInfo,
        tstamp: DateTime<Utc>,
    },
    Offered {
        id: uuid::Uuid,
        bill: BillInfo,
        ttl: DateTime<Utc>,
        keyset_id: cashu::Id,
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
    },
    OfferExpired {
        id: uuid::Uuid,
        bill: BillInfo,
        tstamp: DateTime<Utc>,
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
    },
    Denied {
        id: uuid::Uuid,
        bill: BillInfo,
        tstamp: DateTime<Utc>,
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
        tstamp: DateTime<Utc>,
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

/// Admin quote details plus the immutable Mint-selected credit program binding.
///
/// The fields are optional only so pre-binding quote records remain readable.
/// Operator actions on an unbound quote are rejected by the quote service.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AdminInfoReply {
    #[serde(flatten)]
    pub quote: InfoReply,
    pub credit_program_version: Option<String>,
    pub credit_program_digest: Option<String>,
    pub credit_authorization_receipt: Option<CreditAuthorizationReceipt>,
    pub credit_evidence: Option<MintCreditEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptorRiskEvidence {
    pub schema_version: String,
    pub evidence_id: uuid::Uuid,
    pub signed_evidence: SignedAcceptorRiskEvidence,
    pub operator_id: String,
    pub written_basis_digest: String,
    pub recorded_at: DateTime<Utc>,
    pub verified_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptorRiskAuthorityEvidence {
    pub schema_version: String,
    pub key_id: String,
    pub acceptor_ref: String,
    pub probability_of_default_bps: u32,
    pub loss_given_default_bps: u32,
    pub evidence_state: String,
    pub methodology_version: String,
    pub assessed_by: String,
    pub assessed_at: chrono::NaiveDate,
    pub valid_through: chrono::NaiveDate,
    pub evidence_refs: Vec<String>,
    pub synthetic: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SignedAcceptorRiskEvidence {
    pub evidence: AcceptorRiskAuthorityEvidence,
    pub evidence_digest: String,
    pub signature_algorithm: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MintCreditEvidence {
    pub schema_version: String,
    pub mint_id: String,
    pub acceptor_ref: String,
    pub acceptor_risk: Option<AcceptorRiskEvidence>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptorRiskEvidenceRequest {
    pub signed_evidence: SignedAcceptorRiskEvidence,
    pub written_basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptorRiskEvidenceCommand {
    pub operator_id: String,
    pub request: AcceptorRiskEvidenceRequest,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListPendingQueryRequest {
    pub since: Option<DateTime<Utc>>,
}

/// --------------------------- Update quote status request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase", tag = "action")]
pub enum UpdateQuoteRequest {
    Deny,
    Offer {
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
        ttl: Option<DateTime<Utc>>,
    },
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase", tag = "status")]
pub enum UpdateQuoteResponse {
    Denied,
    Offered {
        #[schema(value_type = u64)]
        discounted: bitcoin::Amount,
        ttl: DateTime<Utc>,
    },
}

/// Exact governed terms signed by AI Credit. Monetary values stay decimal strings so
/// JavaScript and Rust verify the same bytes without an unsafe number conversion.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreditAuthorizationTerms {
    pub bill_sum_sat: String,
    pub discounted_sat: String,
    pub applied_discount_sat: String,
    pub operating_cost_sat: String,
    pub effective_fee_sat: String,
    pub endorsement_exposure_sat: String,
    pub maturity_date: String,
    pub offer_expires_on: String,
    pub tenor_days: u32,
    pub annual_discount_bps: u32,
    pub effective_annual_bps: u32,
    pub fee_ratio_bps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreditAuthorizationEnvelope {
    pub schema_version: String,
    pub key_id: String,
    pub mint_id: String,
    pub mint_quote_id: String,
    pub credit_program_version: String,
    pub credit_program_digest: String,
    pub case_id: String,
    pub bill_id: String,
    pub bill_state_digest: String,
    pub holder_ref: String,
    pub acceptor_ref: String,
    pub decision_snapshot_digest: String,
    pub decision_result_digest: String,
    pub policy_pack_digest: String,
    pub policy_pack_version: String,
    pub calculation_version: String,
    pub terms: CreditAuthorizationTerms,
    pub operator_id: String,
    pub issued_at: String,
    pub expires_at: String,
    pub nonce: String,
    pub action: String,
    pub synthetic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SignedCreditAuthorizationEnvelope {
    pub authorization: CreditAuthorizationEnvelope,
    pub authorization_digest: String,
    pub signature_algorithm: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorizedQuoteRequest {
    pub signed_authorization: SignedCreditAuthorizationEnvelope,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreditAuthorizationReceipt {
    pub receipt_version: String,
    pub operation_id: String,
    pub authorization_digest: String,
    pub case_id: String,
    pub status: String,
    pub mint_id: String,
    pub bill_id: String,
    pub action: String,
    pub effect_id: String,
    pub result_digest: String,
    pub completed_at: String,
    pub synthetic: bool,
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
    use std::str::FromStr;

    use bitcoin::hashes::{Hash as _, sha256};

    use super::*;

    fn reissue_request_fixture() -> ReissueEnquireRequestV1 {
        ReissueEnquireRequestV1 {
            schema_version: REISSUE_ENQUIRE_SCHEMA_VERSION.to_owned(),
            action: REISSUE_ENQUIRE_ACTION.to_owned(),
            content: SharedBill {
                bill_id: BillId::from_str("bitcrt285psGq4Lz4fEQwfM3We5HPznJq8p1YvRaddszFaU5dY")
                    .unwrap(),
                data: "encrypted-bill".to_owned(),
                file_urls: vec![url::Url::parse("https://example.test/evidence.pdf").unwrap()],
                hash: "bill-hash".to_owned(),
                signature: "bill-signature".to_owned(),
                receiver: bitcoin::PublicKey::from_str(
                    "026423b7d36d05b8d50a89a1b4ef2a06c88bcd2c5e650f25e122fa682d3b39686c",
                )
                .unwrap(),
            },
            minting_pubkey: cashu::PublicKey::from_str(
                "026423b7d36d05b8d50a89a1b4ef2a06c88bcd2c5e650f25e122fa682d3b39686c",
            )
            .unwrap(),
            signed_permit: SignedCreditQuoteReissuePermit {
                permit: CreditQuoteReissuePermit {
                    schema_version: "credit-quote-reissue-permit-v1".to_owned(),
                    key_id: "local-testnet-key".to_owned(),
                    mint_id: "local-wildcat".to_owned(),
                    previous_mint_quote_id: uuid::Uuid::parse_str("11111111-1111-4111-8111-111111111111")
                        .unwrap(),
                    reissued_mint_quote_id: uuid::Uuid::parse_str("22222222-2222-4222-8222-222222222222")
                        .unwrap(),
                    credit_program_version: "coffee-v1".to_owned(),
                    credit_program_digest: format!("sha256:{}", "1".repeat(64)),
                    case_id: "case-1".to_owned(),
                    bill_id: "bitcrt285psGq4Lz4fEQwfM3We5HPznJq8p1YvRaddszFaU5dY".to_owned(),
                    bill_state_digest: format!("sha256:{}", "2".repeat(64)),
                    holder_ref: "bitcrt-holder".to_owned(),
                    review_request_id: uuid::Uuid::parse_str("33333333-3333-4333-8333-333333333333")
                        .unwrap(),
                    contested_decision_result_digest: format!("sha256:{}", "3".repeat(64)),
                    corrected_submission_digest: format!("sha256:{}", "4".repeat(64)),
                    issued_at: "2026-08-24T00:00:00.000Z".to_owned(),
                    expires_at: "2026-08-25T00:00:00.000Z".to_owned(),
                    nonce: uuid::Uuid::parse_str("44444444-4444-4444-8444-444444444444")
                        .unwrap(),
                    action: REISSUE_ENQUIRE_ACTION.to_owned(),
                    synthetic: true,
                },
                permit_digest: format!("sha256:{}", "5".repeat(64)),
                signature_algorithm: "Ed25519".to_owned(),
                signature: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=="
                    .to_owned(),
            },
        }
    }

    #[test]
    fn quote_reissue_borsh_contract_fixture() {
        let bytes = borsh::to_vec(&reissue_request_fixture()).unwrap();
        assert_eq!(
            sha256::Hash::hash(&bytes).to_string(),
            "a406744b2d1762df6e6b02f566a9dad2da9bb2fcae5925f9980fad68f6c1a91d"
        );
        let decoded: ReissueEnquireRequestV1 = borsh::from_slice(&bytes).unwrap();
        assert_eq!(decoded.schema_version, REISSUE_ENQUIRE_SCHEMA_VERSION);
        assert_eq!(
            decoded.signed_permit.permit.reissued_mint_quote_id,
            uuid::Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap()
        );
    }

    #[test]
    fn public_quote_projection_contains_only_the_opaque_pointer() {
        let reply = QuoteStatusReply {
            quote: StatusReply::Pending,
            applicant_action: Some(ApplicantActionProjection {
                kind: ApplicantActionKind::Clarification,
                revision_digest: format!("sha256:{}", "a".repeat(64)),
            }),
        };

        let value = serde_json::to_value(reply).unwrap();
        assert_eq!(value["status"], "Pending");
        assert_eq!(value["applicantAction"]["kind"], "clarification");
        assert_eq!(
            value["applicantAction"]["revisionDigest"],
            format!("sha256:{}", "a".repeat(64))
        );
        assert_eq!(value.as_object().unwrap().len(), 2);
        assert_eq!(value["applicantAction"].as_object().unwrap().len(), 2);
        for forbidden in ["questions", "caseId", "token", "applicantId"] {
            assert!(!value.to_string().contains(forbidden));
        }

        let decoded: QuoteStatusReply = serde_json::from_value(value).unwrap();
        assert!(matches!(decoded.quote, StatusReply::Pending));
        let no_action = serde_json::to_value(QuoteStatusReply {
            quote: StatusReply::Pending,
            applicant_action: None,
        })
        .unwrap();
        assert_eq!(no_action, serde_json::json!({ "status": "Pending" }));
    }

    #[test]
    fn applicant_action_receipt_matches_the_strict_bridge_shape() {
        let receipt = CreditApplicantActionReceipt {
            schema_version: String::from("credit-applicant-action-receipt-v1"),
            operation_id: format!("sha256:{}", "1".repeat(64)),
            mint_quote_id: uuid::Uuid::from_u128(1),
            credit_program_version: String::from("synthetic-v1"),
            credit_program_digest: format!("sha256:{}", "2".repeat(64)),
            revision_digest: format!("sha256:{}", "3".repeat(64)),
            expected_revision_digest: None,
            applicant_action: CreditApplicantAction::ClarificationRequired,
            action: String::from("project_applicant_action"),
            status: String::from("completed"),
            completed_at: String::from("2026-08-29T10:00:00.000Z"),
        };
        let value = serde_json::to_value(&receipt).unwrap();
        assert_eq!(value["schemaVersion"], "credit-applicant-action-receipt-v1");
        assert_eq!(value["applicantAction"], "clarification_required");
        assert!(value["expectedRevisionDigest"].is_null());
        assert_eq!(value.as_object().unwrap().len(), 11);
        assert_eq!(
            serde_json::from_value::<CreditApplicantActionReceipt>(value).unwrap(),
            receipt
        );
    }
}
