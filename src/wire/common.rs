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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum ProtestStatus {
    Resolved,
    Rabid,
    Offline,
}

#[cfg(test)]
mod protest_contract {
    use super::*;
    use crate::{
        cashu, ecash,
        wire::{clowder::ClowderRejection, melt, mint, swap, test_utils::PROTEST_CONTRACT},
    };

    fn entry(path: &str) -> serde_json::Value {
        let fixture: serde_json::Value = serde_json::from_str(PROTEST_CONTRACT).expect("fixture");
        fixture
            .pointer(path)
            .unwrap_or_else(|| panic!("fixture has no {path}"))
            .clone()
    }

    fn parse<T: serde::de::DeserializeOwned>(path: &str) -> T {
        serde_json::from_value(entry(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    fn assert_same_json<T: Serialize>(path: &str, value: &T) {
        let serialized = serde_json::to_value(value).expect("serialize");
        assert_eq!(serialized, entry(path), "{path}");
    }

    #[test]
    fn requests_parse_and_serialize_back_unchanged() {
        let request: mint::MintProtestRequest = parse("/mint/request");
        assert_same_json("/mint/request", &request);
        let request: swap::SwapProtestRequest = parse("/swap/request");
        assert_eq!(request.proofs.len(), 1);
        assert_same_json("/swap/request", &request);
        let request: melt::MeltProtestRequest = parse("/melt/request");
        assert_same_json("/melt/request", &request);
    }

    #[test]
    fn melt_outcome_requires_the_txid_only_when_resolved() {
        let txid = "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b";

        let response: melt::MeltProtestResponse = parse("/melt/resolved");
        let outcome = ProtestOutcome::<bitcoin::Txid>::try_from(response).expect("resolved");
        let ProtestOutcome::Resolved(resolved) = outcome.clone() else {
            panic!("expected Resolved, got {outcome:?}");
        };
        assert_eq!(resolved.to_string(), txid);
        let bytes = serde_json::to_string(&melt::MeltProtestResponse::from(outcome)).unwrap();
        assert_eq!(bytes, format!(r#"{{"status":"Resolved","txid":"{txid}"}}"#));

        let response: melt::MeltProtestResponse = parse("/melt/resolved_without_txid");
        assert_eq!(
            ProtestOutcome::<bitcoin::Txid>::try_from(response),
            Err(ProtestOutcomeError::ResolvedWithoutResult)
        );

        let response: melt::MeltProtestResponse = parse("/melt/rabid");
        let outcome = ProtestOutcome::<bitcoin::Txid>::try_from(response).expect("rabid");
        assert_eq!(outcome, ProtestOutcome::Rabid);
        let bytes = serde_json::to_string(&melt::MeltProtestResponse::from(outcome)).unwrap();
        assert_eq!(bytes, r#"{"status":"Rabid","txid":null}"#);

        let response: melt::MeltProtestResponse = parse("/melt/offline");
        assert_eq!(
            ProtestOutcome::<bitcoin::Txid>::try_from(response),
            Ok(ProtestOutcome::Offline)
        );
    }

    #[test]
    fn mint_outcome_carries_signatures_only_when_resolved() {
        let response: mint::MintProtestResponse = parse("/mint/resolved");
        let outcome = ProtestOutcome::<Vec<cashu::BlindSignature>>::try_from(response).unwrap();
        let ProtestOutcome::Resolved(signatures) = &outcome else {
            panic!("expected Resolved, got {outcome:?}");
        };
        assert_eq!(signatures.len(), 1);
        assert_same_json("/mint/resolved", &mint::MintProtestResponse::from(outcome));

        for (path, expected) in [
            ("/mint/rabid", ProtestOutcome::Rabid),
            ("/mint/offline", ProtestOutcome::Offline),
        ] {
            let response: mint::MintProtestResponse = parse(path);
            let outcome = ProtestOutcome::<Vec<cashu::BlindSignature>>::try_from(response);
            assert_eq!(outcome, Ok(expected), "{path}");
            assert_same_json(path, &mint::MintProtestResponse::from(outcome.unwrap()));
        }

        let response: mint::MintProtestResponse = parse("/mint/rabid_with_signatures");
        assert_eq!(
            ProtestOutcome::<Vec<cashu::BlindSignature>>::try_from(response),
            Err(ProtestOutcomeError::UnexpectedResult)
        );
    }

    #[test]
    fn swap_outcome_carries_signatures_only_when_resolved() {
        let response: swap::SwapProtestResponse = parse("/swap/resolved");
        let outcome = ProtestOutcome::<Vec<ecash::BlindSignature>>::try_from(response).unwrap();
        let ProtestOutcome::Resolved(signatures) = &outcome else {
            panic!("expected Resolved, got {outcome:?}");
        };
        assert_eq!(signatures.len(), 1);
        assert_same_json("/swap/resolved", &swap::SwapProtestResponse::from(outcome));

        for (path, expected) in [
            ("/swap/rabid", ProtestOutcome::Rabid),
            ("/swap/offline", ProtestOutcome::Offline),
        ] {
            let response: swap::SwapProtestResponse = parse(path);
            let outcome = ProtestOutcome::<Vec<ecash::BlindSignature>>::try_from(response);
            assert_eq!(outcome, Ok(expected), "{path}");
            assert_same_json(path, &swap::SwapProtestResponse::from(outcome.unwrap()));
        }

        let response: swap::SwapProtestResponse = parse("/swap/resolved_without_signatures");
        assert_eq!(
            ProtestOutcome::<Vec<ecash::BlindSignature>>::try_from(response),
            Err(ProtestOutcomeError::ResolvedWithoutResult)
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
        let cases = [
            (ProtestStatus::Resolved, Some(1), Ok(ProtestOutcome::Resolved(1))),
            (ProtestStatus::Resolved, None, Err(ProtestOutcomeError::ResolvedWithoutResult)),
            (ProtestStatus::Rabid, None, Ok(ProtestOutcome::Rabid)),
            (ProtestStatus::Rabid, Some(1), Err(ProtestOutcomeError::UnexpectedResult)),
            (ProtestStatus::Offline, None, Ok(ProtestOutcome::Offline)),
            (ProtestStatus::Offline, Some(1), Err(ProtestOutcomeError::UnexpectedResult)),
        ];
        for (status, result, expected) in cases {
            let outcome = ProtestOutcome::from_parts(status, result);
            assert_eq!(outcome, expected, "{status:?} {result:?}");
            if let Ok(outcome) = outcome {
                assert_eq!(outcome.into_parts(), (status, result));
            }
        }
    }

    #[test]
    fn rejection_body_is_a_clowder_rejection() {
        assert_eq!(entry("/rejection/http_status"), 400);
        let rejection: ClowderRejection = parse("/rejection/body");
        assert_eq!(
            rejection,
            ClowderRejection::InvalidRequest(
                "wallet signature on the protest does not verify".into()
            )
        );
    }
}
