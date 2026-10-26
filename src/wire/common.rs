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
