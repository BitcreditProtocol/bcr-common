// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::{
    core, ecash,
    wire::borsh::{
        deserialize_from_str, deserialize_optionproofdleq, serialize_as_str,
        serialize_optionproofdleq,
    },
};

// ----- end imports

///--------------------------- New Keyset
#[derive(Serialize, Deserialize, Debug)]
pub struct NewKeysetRequest {
    pub unit: cashu::CurrencyUnit,
    pub expiration: Option<chrono::NaiveDate>,
    pub fees_ppk: u64,
}

///--------------------------- KeysetInfo filters
#[derive(Debug, Default, Deserialize)]
pub struct KeysetInfoFilters {
    pub unit: Option<cashu::CurrencyUnit>,
    pub min_expiration: Option<chrono::NaiveDate>,
    pub max_expiration: Option<chrono::NaiveDate>,
}

///--------------------------- Pre-sign blinded message
#[derive(Serialize, Deserialize, Debug)]
pub struct SignRequest {
    pub kid: cashu::Id,
    pub msg: cashu::BlindedMessage,
}

///--------------------------- Proof fingerprint validation
#[derive(
    Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize, ToSchema, PartialEq,
)]
pub struct ProofFingerprint {
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub keyset_id: cashu::Id,
    pub amount: u64,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub y: cashu::PublicKey, // Y = hash_to_curve(secret)
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub c: cashu::PublicKey, // unblinded signature
    #[borsh(
        serialize_with = "serialize_optionproofdleq",
        deserialize_with = "deserialize_optionproofdleq"
    )]
    pub dleq: Option<cashu::ProofDleq>,
}

impl std::convert::From<ProofFingerprint> for core::signature::ProofFingerprint {
    fn from(fp: ProofFingerprint) -> Self {
        core::signature::ProofFingerprint {
            keyset_id: fp.keyset_id,
            amount: cashu::Amount::from(fp.amount),
            y: *fp.y,
            c: *fp.c,
        }
    }
}

impl std::convert::TryFrom<cashu::Proof> for ProofFingerprint {
    type Error = cashu::nut00::Error;
    fn try_from(proof: cashu::Proof) -> std::result::Result<Self, Self::Error> {
        let y = proof.y()?;
        Ok(ProofFingerprint {
            keyset_id: proof.keyset_id,
            amount: proof.amount.into(),
            y,
            c: proof.c,
            dleq: proof.dleq,
        })
    }
}

pub fn fp_to_proof(fp: &ProofFingerprint, secret: cashu::secret::Secret) -> cashu::Proof {
    cashu::Proof {
        keyset_id: fp.keyset_id,
        amount: cashu::Amount::from(fp.amount),
        c: fp.c,
        dleq: fp.dleq.clone(),
        witness: None,
        secret,
        p2pk_e: None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct KeysetResponse {
    pub keysets: Vec<ecash::KeySetInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_tests;

    #[test]
    fn keyset_response_json_wire_compat() {
        let response = cashu::KeysetResponse {
            keysets: vec![
                core_tests::generate_random_ecash_keyset().0.into(),
                core_tests::generate_random_ecash_keyset().0.into(),
                core_tests::generate_random_ecash_keyset().0.into(),
            ],
        };
        let bytes = serde_json::to_vec(&response).expect("serialize");
        let deserialized: KeysetResponse = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized.keysets.len(), response.keysets.len());
        assert_eq!(deserialized.keysets[0].id, response.keysets[0].id);
        assert_eq!(deserialized.keysets[1].id, response.keysets[1].id);
        assert_eq!(deserialized.keysets[2].id, response.keysets[2].id);
        let deserialized_bytes = serde_json::to_vec(&deserialized).expect("serialize");
        let deserialized2: cashu::KeysetResponse =
            serde_json::from_slice(&deserialized_bytes).expect("deserialize");
        assert_eq!(response, deserialized2);
    }
}
