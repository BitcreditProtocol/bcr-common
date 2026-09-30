// ----- standard library import
// ----- extra library imports
use bitcoin::secp256k1 as secp;
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
    #[serde(default, with = "crate::wire::bill_date::option")]
    pub expiration: Option<time::Date>,
    pub fees_ppk: u64,
}

///--------------------------- KeysetInfo filters
#[derive(Debug, Default, Deserialize)]
pub struct KeysetInfoFilters {
    pub unit: Option<cashu::CurrencyUnit>,
    #[serde(default, with = "crate::wire::bill_date::option")]
    pub min_expiration: Option<time::Date>,
    #[serde(default, with = "crate::wire::bill_date::option")]
    pub max_expiration: Option<time::Date>,
}

///--------------------------- Pre-sign blinded message
#[derive(Serialize, Deserialize, Debug)]
pub struct SignRequest {
    pub kid: ecash::Id,
    pub msg: ecash::BlindedMessage,
}

///--------------------------- Proof fingerprint validation
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq)]
pub struct ProofFingerprint {
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub keyset_id: ecash::Id,
    pub amount: u64,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    #[serde(with = "secp_compat")]
    pub y: secp::PublicKey, // Y = hash_to_curve(secret)
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    #[serde(with = "secp_compat")]
    pub c: secp::PublicKey, // unblinded signature
    #[borsh(
        serialize_with = "serialize_optionproofdleq",
        deserialize_with = "deserialize_optionproofdleq"
    )]
    pub dleq: Option<cashu::ProofDleq>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq)]
pub struct ProofFingerprintV1 {
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub keyset_id: ecash::Id,
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

impl std::convert::From<&ProofFingerprint> for core::signature::ProofFingerprint {
    fn from(fp: &ProofFingerprint) -> Self {
        core::signature::ProofFingerprint {
            keyset_id: fp.keyset_id.into(),
            amount: cashu::Amount::from(fp.amount),
            y: fp.y,
            c: fp.c,
        }
    }
}

impl std::convert::From<ProofFingerprint> for core::signature::ProofFingerprint {
    fn from(fp: ProofFingerprint) -> Self {
        Self::from(&fp)
    }
}

impl ProofFingerprint {
    pub fn verify_dleq(&self, keyset: &ecash::KeySet) -> core::signature::ECashSignatureResult<()> {
        core::signature::verify_fingerprint_dleq(keyset, &self.into(), self.dleq.as_ref())
    }
}

impl std::convert::TryFrom<ecash::Proof> for ProofFingerprint {
    type Error = ecash::Error;
    fn try_from(proof: ecash::Proof) -> std::result::Result<Self, Self::Error> {
        let y = proof.y()?;
        Ok(ProofFingerprint {
            keyset_id: proof.keyset_id,
            amount: proof.amount.into(),
            y: ecash::public_key_cashu2secp(&y),
            c: ecash::public_key_cashu2secp(&proof.c),
            dleq: proof.dleq,
        })
    }
}

impl std::convert::TryFrom<cashu::Proof> for ProofFingerprint {
    type Error = cashu::nut00::Error;
    fn try_from(proof: cashu::Proof) -> std::result::Result<Self, Self::Error> {
        let y = proof.y()?;
        Ok(ProofFingerprint {
            keyset_id: proof.keyset_id.into(),
            amount: proof.amount.into(),
            y: ecash::public_key_cashu2secp(&y),
            c: ecash::public_key_cashu2secp(&proof.c),
            dleq: proof.dleq,
        })
    }
}

pub fn fp_to_proof(fp: &ProofFingerprint, secret: cashu::secret::Secret) -> ecash::Proof {
    ecash::Proof {
        keyset_id: fp.keyset_id,
        amount: cashu::Amount::from(fp.amount),
        c: ecash::public_key_secp2cashu(&fp.c),
        dleq: fp.dleq.clone(),
        witness: None,
        secret,
        p2pk_e: None,
    }
}

pub fn fp_to_cashu_proof(fp: &ProofFingerprint, secret: cashu::secret::Secret) -> cashu::Proof {
    cashu::Proof {
        keyset_id: fp.keyset_id.into(),
        amount: cashu::Amount::from(fp.amount),
        c: ecash::public_key_secp2cashu(&fp.c),
        dleq: fp.dleq.clone(),
        witness: None,
        secret,
        p2pk_e: None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct KeysetInfoListResponse {
    pub keysets: Vec<ecash::KeySetInfo>,
}

mod secp_compat {
    use bitcoin::{hex::prelude::*, secp256k1 as secp};
    use serde::{Deserialize, Deserializer, Serializer};

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_tests;

    fn sample_fingerprint() -> ProofFingerprint {
        let (_, keyset) = core_tests::generate_random_ecash_keyset();
        let proof = core_tests::generate_random_ecash_proofs(&keyset, &[cashu::Amount::from(8u64)])
            .pop()
            .expect("one proof");
        ProofFingerprint::try_from(proof).expect("fingerprint")
    }

    /// The same fingerprint as it was laid out before `y`/`c` became `secp::PublicKey`.
    fn as_v1(fp: &ProofFingerprint) -> ProofFingerprintV1 {
        ProofFingerprintV1 {
            keyset_id: fp.keyset_id,
            amount: fp.amount,
            y: ecash::public_key_secp2cashu(&fp.y),
            c: ecash::public_key_secp2cashu(&fp.c),
            dleq: fp.dleq.clone(),
        }
    }

    #[test]
    fn proof_fingerprint_wire_compat() {
        let fp = sample_fingerprint();
        let v1 = as_v1(&fp);
        // json
        let bytes = serde_json::to_vec(&fp).expect("serialize");
        let v1_bytes = serde_json::to_vec(&v1).expect("serialize");
        assert_eq!(bytes, v1_bytes);
        let deserialized: ProofFingerprintV1 = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized, v1);
        let back: ProofFingerprint = serde_json::from_slice(&v1_bytes).expect("deserialize");
        assert_eq!(back, fp);
        // cbor
        let mut bytes = Vec::new();
        ciborium::into_writer(&fp, &mut bytes).expect("serialize");
        let mut v1_bytes = Vec::new();
        ciborium::into_writer(&v1, &mut v1_bytes).expect("serialize");
        assert_eq!(bytes, v1_bytes);
        let deserialized: ProofFingerprintV1 =
            ciborium::from_reader(bytes.as_slice()).expect("deserialize");
        assert_eq!(deserialized, v1);
        let back: ProofFingerprint =
            ciborium::from_reader(v1_bytes.as_slice()).expect("deserialize");
        assert_eq!(back, fp);
        // borsh
        let bytes = borsh::to_vec(&fp).expect("serialize");
        let v1_bytes = borsh::to_vec(&v1).expect("serialize");
        assert_eq!(bytes, v1_bytes);
        let deserialized: ProofFingerprintV1 = borsh::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized, v1);
        let back: ProofFingerprint = borsh::from_slice(&v1_bytes).expect("deserialize");
        assert_eq!(back, fp);
    }

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
        let deserialized: KeysetInfoListResponse =
            serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized.keysets.len(), response.keysets.len());
        assert_eq!(
            cashu::Id::from(deserialized.keysets[0].id),
            response.keysets[0].id
        );
        assert_eq!(
            cashu::Id::from(deserialized.keysets[1].id),
            response.keysets[1].id
        );
        assert_eq!(
            cashu::Id::from(deserialized.keysets[2].id),
            response.keysets[2].id
        );
        let deserialized_bytes = serde_json::to_vec(&deserialized).expect("serialize");
        let deserialized2: cashu::KeysetResponse =
            serde_json::from_slice(&deserialized_bytes).expect("deserialize");
        assert_eq!(response, deserialized2);
    }
}
