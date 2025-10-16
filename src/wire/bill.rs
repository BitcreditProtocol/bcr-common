// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::{
    core::{BillId, NodeId},
    wire::{
        borsh::{deserialize_vec_url, serialize_vec_url},
        contact::ContactType,
        identity::{File, PostalAddress},
    },
};
// ----- end imports

#[derive(Debug, Serialize, Deserialize)]
pub struct BillsResponse<T: Serialize> {
    pub bills: Vec<T>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BitcreditBill {
    #[schema(value_type=String)]
    pub id: BillId,
    pub participants: BillParticipants,
    pub data: BillData,
    pub status: BillStatus,
    pub current_waiting_state: Option<BillCurrentWaitingState>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum BillCurrentWaitingState {
    Sell(BillWaitingForSellState),
    Payment(BillWaitingForPaymentState),
    Recourse(BillWaitingForRecourseState),
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillWaitingForSellState {
    pub buyer: BillParticipant,
    pub seller: BillParticipant,
    pub payment_data: BillWaitingStatePaymentData,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillWaitingForPaymentState {
    pub payer: BillIdentParticipant,
    pub payee: BillParticipant,
    pub payment_data: BillWaitingStatePaymentData,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillWaitingForRecourseState {
    pub recourser: BillParticipant,
    pub recoursee: BillIdentParticipant,
    pub payment_data: BillWaitingStatePaymentData,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillWaitingStatePaymentData {
    pub time_of_request: u64,
    pub currency: String,
    pub sum: String,
    pub link_to_pay: String,
    pub address_to_pay: String,
    pub mempool_link_for_address_to_pay: String,
    pub tx_id: Option<String>,
    pub in_mempool: bool,
    pub confirmations: u64,
    pub payment_deadline: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillStatus {
    pub acceptance: BillAcceptanceStatus,
    pub payment: BillPaymentStatus,
    pub sell: BillSellStatus,
    pub recourse: BillRecourseStatus,
    pub mint: BillMintStatus,
    pub redeemed_funds_available: bool,
    pub has_requested_funds: bool,
    pub last_block_time: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillAcceptanceStatus {
    pub time_of_request_to_accept: Option<u64>,
    pub requested_to_accept: bool,
    pub accepted: bool,
    pub request_to_accept_timed_out: bool,
    pub rejected_to_accept: bool,
    pub acceptance_deadline_timestamp: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillPaymentStatus {
    pub time_of_request_to_pay: Option<u64>,
    pub requested_to_pay: bool,
    pub paid: bool,
    pub request_to_pay_timed_out: bool,
    pub rejected_to_pay: bool,
    pub payment_deadline_timestamp: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillSellStatus {
    pub time_of_last_offer_to_sell: Option<u64>,
    pub sold: bool,
    pub offered_to_sell: bool,
    pub offer_to_sell_timed_out: bool,
    pub rejected_offer_to_sell: bool,
    pub buying_deadline_timestamp: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillRecourseStatus {
    pub time_of_last_request_to_recourse: Option<u64>,
    pub recoursed: bool,
    pub requested_to_recourse: bool,
    pub request_to_recourse_timed_out: bool,
    pub rejected_request_to_recourse: bool,
    pub recourse_deadline_timestamp: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillMintStatus {
    pub has_mint_requests: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillData {
    pub time_of_drawing: u64,
    pub issue_date: chrono::NaiveDate,
    pub time_of_maturity: u64,
    pub maturity_date: chrono::NaiveDate,
    pub country_of_issuing: String,
    pub city_of_issuing: String,
    pub country_of_payment: String,
    pub city_of_payment: String,
    pub currency: String,
    pub sum: String,
    pub files: Vec<File>,
    pub active_notification: Option<Notification>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillParticipants {
    pub drawee: BillIdentParticipant,
    pub drawer: BillIdentParticipant,
    pub payee: BillParticipant,
    pub endorsee: Option<BillParticipant>,
    pub endorsements: Vec<Endorsement>,
    pub endorsements_count: u64,
    #[schema(value_type=Vec<String>)]
    pub all_participant_node_ids: Vec<NodeId>,
}

#[derive(Debug, Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, ToSchema)]
pub enum BillParticipant {
    Anon(BillAnonParticipant),
    Ident(BillIdentParticipant),
}

impl BillParticipant {
    pub fn node_id(&self) -> NodeId {
        match self {
            BillParticipant::Ident(data) => data.node_id.clone(),
            BillParticipant::Anon(data) => data.node_id.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, ToSchema)]
pub struct BillAnonParticipant {
    #[schema(value_type=String)]
    pub node_id: NodeId,
    pub email: Option<String>,
    #[schema(value_type=Vec<String>)]
    #[borsh(
        serialize_with = "serialize_vec_url",
        deserialize_with = "deserialize_vec_url"
    )]
    pub nostr_relays: Vec<url::Url>,
}

#[derive(Debug, Serialize, Deserialize, Clone, BorshSerialize, BorshDeserialize, ToSchema)]
pub struct BillIdentParticipant {
    #[serde(rename = "type")]
    pub t: ContactType,
    #[schema(value_type=String)]
    pub node_id: NodeId,
    pub name: String,
    #[serde(flatten)]
    pub postal_address: PostalAddress,
    pub email: Option<String>,
    #[schema(value_type=Vec<String>)]
    #[borsh(
        serialize_with = "serialize_vec_url",
        deserialize_with = "deserialize_vec_url"
    )]
    pub nostr_relays: Vec<url::Url>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Notification {
    pub id: String,
    #[schema(value_type=Option<String>)]
    pub node_id: Option<NodeId>,
    pub notification_type: NotificationType,
    pub reference_id: Option<String>,
    pub description: String,
    pub datetime: chrono::DateTime<chrono::Utc>,
    pub active: bool,
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum NotificationType {
    General,
    Bill,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestToPayBitcreditBillPayload {
    pub bill_id: BillId,
    pub currency: String,
    pub deadline: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BillCombinedBitcoinKey {
    pub private_descriptor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Endorsement {
    pub pay_to_the_order_of: LightBillParticipant,
    pub signed: LightSignedBy,
    pub signing_timestamp: u64,
    pub signing_address: Option<PostalAddress>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct LightSignedBy {
    pub data: LightBillParticipant,
    pub signatory: Option<LightBillIdentParticipant>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub enum LightBillParticipant {
    Anon(LightBillAnonParticipant),
    Ident(LightBillIdentParticipantWithAddress),
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct LightBillAnonParticipant {
    #[schema(value_type=String)]
    pub node_id: NodeId,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct LightBillIdentParticipant {
    #[serde(rename = "type")]
    pub t: ContactType,
    pub name: String,
    #[schema(value_type=String)]
    pub node_id: NodeId,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct LightBillIdentParticipantWithAddress {
    #[serde(rename = "type")]
    pub t: ContactType,
    pub name: String,
    #[schema(value_type=String)]
    pub node_id: NodeId,
    #[serde(flatten)]
    pub postal_address: PostalAddress,
}
