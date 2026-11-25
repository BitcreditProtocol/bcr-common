// ----- standard library imports
// ----- extra library imports
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
// ----- local imports

// ----- end imports

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T: ToSchema> {
    pub data: Vec<T>,
    pub total: u64,
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
pub struct Pagination {
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ProtestStatus {
    Resolved,
    Rabid,
    Offline,
}

/// The answer to a protest, with the result the alpha issued when it is resolved
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtestOutcome<T> {
    /// the result of the committed request, found in the ledger or returned by the replay
    Resolved(T),
    /// the alpha explicitly rejected a valid committed request
    Rabid,
    /// the alpha could not be reached, so it is not marked rabid
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ProtestOutcomeError {
    #[error("protest resolved without its result")]
    ResolvedWithoutResult,
    #[error("protest that is not resolved carries a result")]
    UnexpectedResult,
}

impl<T> ProtestOutcome<T> {
    /// A result is present if and only if the status is `Resolved`
    pub fn from_parts(
        status: ProtestStatus,
        result: Option<T>,
    ) -> Result<Self, ProtestOutcomeError> {
        match (status, result) {
            (ProtestStatus::Resolved, Some(result)) => Ok(Self::Resolved(result)),
            (ProtestStatus::Resolved, None) => Err(ProtestOutcomeError::ResolvedWithoutResult),
            (ProtestStatus::Rabid, None) => Ok(Self::Rabid),
            (ProtestStatus::Offline, None) => Ok(Self::Offline),
            (ProtestStatus::Rabid | ProtestStatus::Offline, Some(_)) => {
                Err(ProtestOutcomeError::UnexpectedResult)
            }
        }
    }

    pub fn into_parts(self) -> (ProtestStatus, Option<T>) {
        match self {
            Self::Resolved(result) => (ProtestStatus::Resolved, Some(result)),
            Self::Rabid => (ProtestStatus::Rabid, None),
            Self::Offline => (ProtestStatus::Offline, None),
        }
    }
}

/// Asserts a migrated wire type still encodes exactly as the shape it replaced:
/// byte-identical both ways, in json and in cbor. cbor is the one that catches a
/// `secp::PublicKey` left without [`secp_compat`], since it is the only format here
/// that reports `is_human_readable() == false`.
#[cfg(test)]
pub(crate) fn assert_wire_compat<T, V1>(value: &T, v1: &V1)
where
    T: Serialize + serde::de::DeserializeOwned,
    V1: std::fmt::Debug + PartialEq + Serialize + serde::de::DeserializeOwned,
{
    // json
    let bytes = serde_json::to_vec(value).expect("serialize");
    let v1_bytes = serde_json::to_vec(v1).expect("serialize");
    assert_eq!(bytes, v1_bytes, "json encodings diverge");
    let as_v1: V1 = serde_json::from_slice(&bytes).expect("v1 reads the new encoding");
    assert_eq!(&as_v1, v1);
    let back: T = serde_json::from_slice(&v1_bytes).expect("the new type reads a v1 blob");
    assert_eq!(
        serde_json::to_vec(&back).expect("serialize"),
        bytes,
        "a v1 blob does not round-trip through the new type"
    );
    // cbor
    let mut bytes = Vec::new();
    ciborium::into_writer(value, &mut bytes).expect("serialize");
    let mut v1_bytes = Vec::new();
    ciborium::into_writer(v1, &mut v1_bytes).expect("serialize");
    assert_eq!(bytes, v1_bytes, "cbor encodings diverge");
    let as_v1: V1 = ciborium::from_reader(bytes.as_slice()).expect("v1 reads the new encoding");
    assert_eq!(&as_v1, v1);
    let back: T = ciborium::from_reader(v1_bytes.as_slice()).expect("the new type reads a v1 blob");
    let mut back_bytes = Vec::new();
    ciborium::into_writer(&back, &mut back_bytes).expect("serialize");
    assert_eq!(
        back_bytes, bytes,
        "a v1 blob does not round-trip through the new type"
    );
}

/// `secp::PublicKey` encoded the way `cashu::PublicKey` encodes: always the
/// compressed-hex string, in every format. Without it `secp::PublicKey` follows
/// `is_human_readable()` and turns into a 33-byte sequence under cbor, which would
/// not read back the messages already stored.
pub mod secp_compat {
    // ----- standard library imports
    use std::collections::BTreeMap;
    // ----- extra library imports
    use bitcoin::{hex::prelude::*, secp256k1 as secp};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    // ----- local imports

    // ----- end imports

    pub fn serialize<S>(key: &secp::PublicKey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let stri = key.serialize().as_hex().to_string();
        serializer.serialize_str(&stri)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<secp::PublicKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let stri: String = Deserialize::deserialize(deserializer)?;
        let bytes: [u8; 33] = FromHex::from_hex(&stri).map_err(serde::de::Error::custom)?;
        let key = secp::PublicKey::from_slice(&bytes).map_err(serde::de::Error::custom)?;
        Ok(key)
    }

    #[derive(Serialize, Deserialize)]
    struct Hex(#[serde(with = "super::secp_compat")] secp::PublicKey);

    /// `#[serde(with = "...::secp_compat::vec")]` for `Vec<secp::PublicKey>`
    pub mod vec {
        use super::*;

        pub fn serialize<S>(keys: &[secp::PublicKey], serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.collect_seq(keys.iter().copied().map(Hex))
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<secp::PublicKey>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let keys: Vec<Hex> = Deserialize::deserialize(deserializer)?;
            Ok(keys.into_iter().map(|Hex(key)| key).collect())
        }
    }

    /// `#[serde(with = "...::secp_compat::btreemap_values")]` for a `BTreeMap` whose
    /// values are `secp::PublicKey`
    pub mod btreemap_values {
        use super::*;

        pub fn serialize<K, S>(
            keys: &BTreeMap<K, secp::PublicKey>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            K: Serialize,
            S: Serializer,
        {
            serializer.collect_map(keys.iter().map(|(k, v)| (k, Hex(*v))))
        }

        pub fn deserialize<'de, K, D>(
            deserializer: D,
        ) -> Result<BTreeMap<K, secp::PublicKey>, D::Error>
        where
            K: Deserialize<'de> + Ord,
            D: Deserializer<'de>,
        {
            let keys: BTreeMap<K, Hex> = Deserialize::deserialize(deserializer)?;
            Ok(keys.into_iter().map(|(k, Hex(v))| (k, v)).collect())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::{cashu, core, wire::common::assert_wire_compat};

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct One {
            #[serde(with = "super")]
            key: secp::PublicKey,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct OneV1 {
            key: cashu::PublicKey,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Many {
            #[serde(with = "super::vec")]
            keys: Vec<secp::PublicKey>,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct ManyV1 {
            keys: Vec<cashu::PublicKey>,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Mapped {
            #[serde(with = "super::btreemap_values")]
            keys: BTreeMap<u64, secp::PublicKey>,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct MappedV1 {
            keys: BTreeMap<u64, cashu::PublicKey>,
        }

        fn sample_keys(count: usize) -> Vec<secp::PublicKey> {
            (0..count)
                .map(|_| core::generate_random_keypair().public_key())
                .collect()
        }

        #[test]
        fn secp_compat_wire_compat() {
            let key = sample_keys(1)[0];
            assert_wire_compat(
                &One { key },
                &OneV1 {
                    key: cashu::PublicKey::from(key),
                },
            );
        }

        #[test]
        fn secp_compat_vec_wire_compat() {
            let keys = sample_keys(3);
            assert_wire_compat(
                &Many { keys: keys.clone() },
                &ManyV1 {
                    keys: keys.iter().copied().map(cashu::PublicKey::from).collect(),
                },
            );
        }

        #[test]
        fn secp_compat_vec_empty_wire_compat() {
            assert_wire_compat(&Many { keys: vec![] }, &ManyV1 { keys: vec![] });
        }

        #[test]
        fn secp_compat_btreemap_values_wire_compat() {
            let keys: BTreeMap<u64, secp::PublicKey> =
                [1u64, 2, 4, 8].into_iter().zip(sample_keys(4)).collect();
            assert_wire_compat(
                &Mapped { keys: keys.clone() },
                &MappedV1 {
                    keys: keys
                        .iter()
                        .map(|(amount, key)| (*amount, cashu::PublicKey::from(*key)))
                        .collect(),
                },
            );
        }

        #[test]
        fn secp_compat_rejects_a_non_key_string() {
            let err = serde_json::from_str::<One>(r#"{"key":"not a public key"}"#);
            assert!(err.is_err());
        }
    }
}

#[cfg(test)]
mod protest_contract {
    use super::*;
    use crate::{
        cashu, ecash,
        wire::{
            melt, mint, swap,
            test_utils::{protest_entry as entry, protest_parse as parse},
        },
    };

    fn assert_same_json<T: Serialize>(path: &str, value: &T) {
        let serialized = serde_json::to_value(value).expect("serialize");
        assert_eq!(serialized, entry(path), "{path}");
    }

    #[test]
    fn requests_parse_and_serialize_back_unchanged() {
        assert_same_json(
            "/mint/request",
            &parse::<mint::MintProtestRequest>("/mint/request"),
        );
        assert_same_json(
            "/swap/request",
            &parse::<swap::SwapProtestRequest>("/swap/request"),
        );
        let request: melt::MeltProtestRequest = parse("/melt/request");
        assert!(request.inputs.is_none());
        assert!(!serde_json::to_string(&request).unwrap().contains("inputs"));
        assert_same_json("/melt/request", &request);
        let request: melt::MeltProtestRequest = parse("/melt/request_with_inputs");
        assert_eq!(request.inputs.as_ref().map(Vec::len), Some(1));
        assert_same_json("/melt/request_with_inputs", &request);
    }

    fn assert_outcomes<R, T>(op: &str, refused: &str, error: ProtestOutcomeError)
    where
        R: serde::de::DeserializeOwned + Serialize + From<ProtestOutcome<T>>,
        ProtestOutcome<T>: TryFrom<R, Error = ProtestOutcomeError>,
        T: std::fmt::Debug + PartialEq,
    {
        let outcome = |name: &str| ProtestOutcome::try_from(parse::<R>(&format!("/{op}/{name}")));
        let resolved = outcome("resolved").expect("resolved");
        assert!(matches!(resolved, ProtestOutcome::Resolved(_)), "{op}");
        assert_same_json(&format!("/{op}/resolved"), &R::from(resolved));
        assert_eq!(outcome("rabid"), Ok(ProtestOutcome::Rabid), "{op}");
        assert_same_json(&format!("/{op}/rabid"), &R::from(ProtestOutcome::Rabid));
        assert_eq!(outcome("offline"), Ok(ProtestOutcome::Offline), "{op}");
        assert_eq!(outcome(refused), Err(error), "{op}");
    }

    #[test]
    fn responses_carry_a_result_only_when_resolved() {
        assert_outcomes::<mint::MintProtestResponse, Vec<cashu::BlindSignature>>(
            "mint",
            "rabid_with_signatures",
            ProtestOutcomeError::UnexpectedResult,
        );
        assert_outcomes::<swap::SwapProtestResponse, Vec<ecash::BlindSignature>>(
            "swap",
            "resolved_without_signatures",
            ProtestOutcomeError::ResolvedWithoutResult,
        );
        assert_outcomes::<melt::MeltProtestResponse, bitcoin::Txid>(
            "melt",
            "resolved_without_txid",
            ProtestOutcomeError::ResolvedWithoutResult,
        );
    }

    #[test]
    fn empty_signatures_are_no_result() {
        let response = swap::SwapProtestResponse {
            status: ProtestStatus::Resolved,
            signatures: Some(vec![]),
        };
        assert_eq!(
            ProtestOutcome::<Vec<ecash::BlindSignature>>::try_from(response),
            Err(ProtestOutcomeError::ResolvedWithoutResult)
        );
        let response = mint::MintProtestResponse {
            status: ProtestStatus::Rabid,
            signatures: Some(vec![]),
        };
        assert_eq!(
            ProtestOutcome::<Vec<cashu::BlindSignature>>::try_from(response),
            Ok(ProtestOutcome::Rabid)
        );
    }

    #[test]
    fn result_is_present_exactly_when_resolved() {
        use ProtestOutcomeError::*;
        use ProtestStatus::*;
        let cases = [
            (Resolved, Some(1), Ok(ProtestOutcome::Resolved(1))),
            (Resolved, None, Err(ResolvedWithoutResult)),
            (Rabid, None, Ok(ProtestOutcome::Rabid)),
            (Rabid, Some(1), Err(UnexpectedResult)),
            (Offline, None, Ok(ProtestOutcome::Offline)),
            (Offline, Some(1), Err(UnexpectedResult)),
        ];
        for (status, result, expected) in cases {
            let outcome = ProtestOutcome::from_parts(status, result);
            assert_eq!(outcome, expected, "{status:?} {result:?}");
            if let Ok(outcome) = outcome {
                assert_eq!(outcome.into_parts(), (status, result));
            }
        }
    }
}
