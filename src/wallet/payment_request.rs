use crate::{
    cashu::{self, Amount, CurrencyUnit, MintUrl, Proof},
    core::NodeId,
    wire::borsh::{
        deserialize_from_str, deserialize_from_u64, deserialize_vecof_cdkproof, serialize_as_str,
        serialize_as_u64, serialize_vecof_cdkproof,
    },
};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const DEFAULT_EVENT_VERSION: &str = "1.0";

fn get_version(_event_type: &EventType) -> String {
    DEFAULT_EVENT_VERSION.into()
}

#[derive(
    strum::VariantArray,
    strum::Display,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    PartialEq,
    BorshSerialize,
    BorshDeserialize,
)]
pub enum EventType {
    ContactPayment,
    ContactPaymentRequest,
}

#[derive(Debug, Clone, BorshSerialize)]
pub struct Event<T: BorshSerialize> {
    pub event_type: EventType,
    pub version: String,
    pub data: T,
}

impl<T: BorshSerialize> Event<T> {
    pub fn new(event_type: EventType, data: T) -> Self {
        Self {
            event_type: event_type.to_owned(),
            version: get_version(&event_type),
            data,
        }
    }

    pub fn new_contact_payment(data: T) -> Self {
        Self::new(EventType::ContactPayment, data)
    }

    pub fn new_contact_payment_request(data: T) -> Self {
        Self::new(EventType::ContactPaymentRequest, data)
    }
}

impl<T: BorshSerialize> TryFrom<Event<T>> for EventEnvelope {
    type Error = std::io::Error;

    fn try_from(event: Event<T>) -> Result<Self, Self::Error> {
        let serialized = &borsh::to_vec(&event.data)?;
        Ok(Self {
            event_type: event.event_type,
            version: event.version,
            data: serialized.to_vec(),
        })
    }
}

impl<T: BorshDeserialize + BorshSerialize> TryFrom<EventEnvelope> for Event<T> {
    type Error = std::io::Error;
    fn try_from(envelope: EventEnvelope) -> Result<Self, Self::Error> {
        let data: T = borsh::from_slice(&envelope.data)?;
        Ok(Self {
            event_type: envelope.event_type,
            version: envelope.version,
            data,
        })
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct EventEnvelope {
    pub event_type: EventType,
    pub version: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ContactPaymentPayload {
    pub payment_request_id: Option<Uuid>,
    pub sender: NodeId,
    #[borsh(
        serialize_with = "serialize_vecof_cdkproof",
        deserialize_with = "deserialize_vecof_cdkproof"
    )]
    pub proofs: Vec<Proof>,
    pub memo: Option<String>,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub mint: MintUrl,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub unit: CurrencyUnit,
    pub created_at: u64,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ContactPaymentRequestPayload {
    pub id: Uuid,
    pub sender: NodeId,
    pub memo: Option<String>,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub mint: MintUrl,
    #[borsh(
        serialize_with = "serialize_as_u64",
        deserialize_with = "deserialize_from_u64"
    )]
    pub amount: cashu::Amount,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub unit: CurrencyUnit,
    pub deadline: Option<u64>,
    pub created_at: u64,
}

impl ContactPaymentRequestPayload {
    pub fn new(
        node_id: NodeId,
        amount: Amount,
        unit: CurrencyUnit,
        memo: Option<String>,
        deadline: Option<u64>,
        mint: MintUrl,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender: node_id,
            memo,
            mint,
            amount,
            unit,
            deadline,
            created_at: time::OffsetDateTime::now_utc().unix_timestamp() as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // Encoded by Wallet-Core's own event types before they moved here.
    const PAYMENT_FIXTURE: &str = "0003000000312e3011010000010123456789abcdef0123456789abcdef480000006269746372743032323935666235663465656232663231653031656166336132643961336265313066333964623837306432386630323134363133303331373937336134306163300100000008000000000000000800000000ad268c4d1f5826400000003430373931356263323132626536316137376533653664326165623463373237393830626461353163643036613661666332396532383631373638613738333702bc9097997d81afb2cc7346b5e4345a9346bd2a506eb7958598a72f0cf85163ea000001050000006c756e63681800000068747470733a2f2f6d696e742e6578616d706c652e636f6d0300000073617400f1536500000000";
    const REQUEST_FIXTURE: &str = "0103000000312e30990000000123456789abcdef0123456789abcdef48000000626974637274303232393566623566346565623266323165303165616633613264396133626531306633396462383730643238663032313436313330333137393733613430616330001800000068747470733a2f2f6d696e742e6578616d706c652e636f6d1500000000000000030000007361740110ff53650000000000f1536500000000";
    const NODE: &str = "bitcrt02295fb5f4eeb2f21e01eaf3a2d9a3be10f39db870d28f02146130317973a40ac0";
    const PROOF: &str = r#"{"amount":8,"id":"00ad268c4d1f5826","secret":"407915bc212be61a77e3e6d2aeb4c727980bda51cd06a6afc29e2861768a7837","C":"02bc9097997d81afb2cc7346b5e4345a9346bd2a506eb7958598a72f0cf85163ea"}"#;

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    fn fixture_id() -> Uuid {
        Uuid::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef)
    }

    fn fixture_mint() -> MintUrl {
        MintUrl::from_str("https://mint.example.com").unwrap()
    }

    #[test]
    fn payment_request_decodes_wallet_core_payment_fixture() {
        let bytes = unhex(PAYMENT_FIXTURE);
        let envelope: EventEnvelope = borsh::from_slice(&bytes).unwrap();
        assert_eq!(envelope.event_type, EventType::ContactPayment);
        assert_eq!(envelope.version, DEFAULT_EVENT_VERSION);
        let event: Event<ContactPaymentPayload> = envelope.try_into().unwrap();
        let payload = event.data.clone();
        assert_eq!(payload.payment_request_id, Some(fixture_id()));
        assert_eq!(payload.sender, NodeId::from_str(NODE).unwrap());
        let proof: Proof = serde_json::from_str(PROOF).unwrap();
        assert_eq!(payload.proofs, vec![proof]);
        assert_eq!(payload.memo.as_deref(), Some("lunch"));
        assert_eq!(payload.mint, fixture_mint());
        assert_eq!(payload.unit, CurrencyUnit::Sat);
        assert_eq!(payload.created_at, 1_700_000_000);

        let reencoded: EventEnvelope = event.try_into().unwrap();
        assert_eq!(borsh::to_vec(&reencoded).unwrap(), bytes);
    }

    #[test]
    fn payment_request_decodes_wallet_core_request_fixture() {
        let bytes = unhex(REQUEST_FIXTURE);
        let envelope: EventEnvelope = borsh::from_slice(&bytes).unwrap();
        assert_eq!(envelope.event_type, EventType::ContactPaymentRequest);
        let event: Event<ContactPaymentRequestPayload> = envelope.try_into().unwrap();
        let payload = event.data.clone();
        assert_eq!(payload.id, fixture_id());
        assert_eq!(payload.sender, NodeId::from_str(NODE).unwrap());
        assert_eq!(payload.memo, None);
        assert_eq!(payload.mint, fixture_mint());
        assert_eq!(payload.amount, Amount::from(21u64));
        assert_eq!(payload.unit, CurrencyUnit::Sat);
        assert_eq!(payload.deadline, Some(1_700_003_600));
        assert_eq!(payload.created_at, 1_700_000_000);

        let reencoded: EventEnvelope = event.try_into().unwrap();
        assert_eq!(borsh::to_vec(&reencoded).unwrap(), bytes);
    }

    #[test]
    fn payment_request_new_round_trips() {
        let request = ContactPaymentRequestPayload::new(
            NodeId::from_str(NODE).unwrap(),
            Amount::from(1000u64),
            CurrencyUnit::Sat,
            Some("rent".into()),
            None,
            fixture_mint(),
        );
        let envelope: EventEnvelope = Event::new_contact_payment_request(request.clone())
            .try_into()
            .unwrap();
        let bytes = borsh::to_vec(&envelope).unwrap();
        let decoded: Event<ContactPaymentRequestPayload> =
            borsh::from_slice::<EventEnvelope>(&bytes)
                .unwrap()
                .try_into()
                .unwrap();
        assert_eq!(decoded.event_type, EventType::ContactPaymentRequest);
        assert_eq!(decoded.data.id, request.id);
        assert_eq!(decoded.data.amount, request.amount);
        assert_eq!(decoded.data.memo, request.memo);
        assert_eq!(decoded.data.created_at, request.created_at);
    }

    #[test]
    fn payment_request_rejects_mismatched_payload() {
        let envelope: EventEnvelope = borsh::from_slice(&unhex(REQUEST_FIXTURE)).unwrap();
        let decoded: Result<Event<ContactPaymentPayload>, _> = envelope.try_into();
        assert!(decoded.is_err());
    }

    // Encoded by bcr_wallet_core::event at Wallet-Core fe1de1d.
    const EXAMPLE_REQUEST_FIXTURE: &str = "0103000000312e309b0000007f1c2d3e4b5a4c6d8e9f0a1b2c3d4e5f480000006269746372743033393138306331363965356636643763353739636631636566613337626666643437613262333839633831323536303166343036386338376265613739353934330106000000636f666665651800000068747470733a2f2f6d696e742e6578616d706c652e636f6de8030000000000000300000073617400803bb16a00000000";
    const EXAMPLE_PAYMENT_FIXTURE: &str = "0003000000312e308d000000017f1c2d3e4b5a4c6d8e9f0a1b2c3d4e5f4800000062697463727430333931383063313639653566366437633537396366316365666133376266666434376132623338396338313235363031663430363863383762656137393539343300000000001800000068747470733a2f2f6d696e742e6578616d706c652e636f6d03000000736174803bb16a00000000";
    const EXAMPLE_NODE: &str =
        "bitcrt039180c169e5f6d7c579cf1cefa37bffd47a2b389c8125601f4068c87bea795943";

    #[test]
    fn payment_request_matches_wallet_core_wire_format() {
        let id = Uuid::from_str("7f1c2d3e-4b5a-4c6d-8e9f-0a1b2c3d4e5f").unwrap();
        let sender = NodeId::from_str(EXAMPLE_NODE).unwrap();

        let request = ContactPaymentRequestPayload {
            id,
            sender: sender.clone(),
            memo: Some("coffee".into()),
            mint: fixture_mint(),
            amount: Amount::from(1000u64),
            unit: CurrencyUnit::Sat,
            deadline: None,
            created_at: 1_790_000_000,
        };
        let envelope =
            EventEnvelope::try_from(Event::new_contact_payment_request(request)).unwrap();
        let bytes = borsh::to_vec(&envelope).unwrap();
        assert_eq!(bytes.len(), 167);
        assert_eq!(bytes, unhex(EXAMPLE_REQUEST_FIXTURE));

        let decoded: Event<ContactPaymentRequestPayload> =
            borsh::from_slice::<EventEnvelope>(&bytes)
                .unwrap()
                .try_into()
                .unwrap();
        assert_eq!(decoded.event_type, EventType::ContactPaymentRequest);
        assert_eq!(decoded.data.id, id);
        assert_eq!(decoded.data.sender.to_string(), EXAMPLE_NODE);
        assert_eq!(decoded.data.memo.as_deref(), Some("coffee"));
        assert_eq!(decoded.data.mint, fixture_mint());
        assert_eq!(decoded.data.amount, Amount::from(1000u64));
        assert_eq!(decoded.data.unit, CurrencyUnit::Sat);
        assert_eq!(decoded.data.deadline, None);
        assert_eq!(decoded.data.created_at, 1_790_000_000);
        let reencoded = EventEnvelope::try_from(decoded).unwrap();
        assert_eq!(borsh::to_vec(&reencoded).unwrap(), bytes);

        let payment = ContactPaymentPayload {
            payment_request_id: Some(id),
            sender,
            proofs: vec![],
            memo: None,
            mint: fixture_mint(),
            unit: CurrencyUnit::Sat,
            created_at: 1_790_000_000,
        };
        let envelope = EventEnvelope::try_from(Event::new_contact_payment(payment)).unwrap();
        let bytes = borsh::to_vec(&envelope).unwrap();
        assert_eq!(bytes[0], 0);
        assert_eq!(bytes, unhex(EXAMPLE_PAYMENT_FIXTURE));
        let decoded: Event<ContactPaymentPayload> = borsh::from_slice::<EventEnvelope>(&bytes)
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(decoded.data.payment_request_id, Some(id));
        assert!(decoded.data.proofs.is_empty());
        let reencoded = EventEnvelope::try_from(decoded).unwrap();
        assert_eq!(borsh::to_vec(&reencoded).unwrap(), bytes);
    }
}
