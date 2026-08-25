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
    pub acceptor_ref: String,
    pub probability_of_default_bps: u32,
    pub loss_given_default_bps: u32,
    pub evidence_state: String,
    pub methodology_version: String,
    pub assessed_by: String,
    pub assessed_at: chrono::NaiveDate,
    pub valid_through: chrono::NaiveDate,
    pub evidence_refs: Vec<String>,
    pub operator_id: String,
    pub recorded_at: DateTime<Utc>,
    pub synthetic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MintCapacityEvidence {
    pub schema_version: String,
    pub evidence_id: uuid::Uuid,
    pub mint_id: String,
    pub existing_exposure_sat: String,
    pub exposure_limit_sat: String,
    pub evidence_state: String,
    pub methodology_version: String,
    pub assessed_by: String,
    pub assessed_at: chrono::NaiveDate,
    pub valid_through: chrono::NaiveDate,
    pub evidence_refs: Vec<String>,
    pub operator_id: String,
    pub recorded_at: DateTime<Utc>,
    pub synthetic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MintCreditEvidence {
    pub schema_version: String,
    pub mint_id: String,
    pub acceptor_ref: String,
    pub acceptor_risk: Option<AcceptorRiskEvidence>,
    pub mint_capacity: Option<MintCapacityEvidence>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptorRiskEvidenceRequest {
    pub probability_of_default_bps: u32,
    pub loss_given_default_bps: u32,
    pub source_reference: String,
    pub valid_through: chrono::NaiveDate,
    pub written_basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptorRiskEvidenceCommand {
    pub operator_id: String,
    pub request: AcceptorRiskEvidenceRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MintCapacityEvidenceRequest {
    pub existing_exposure_sat: String,
    pub exposure_limit_sat: String,
    pub source_reference: String,
    pub valid_through: chrono::NaiveDate,
    pub written_basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MintCapacityEvidenceCommand {
    pub operator_id: String,
    pub request: MintCapacityEvidenceRequest,
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
