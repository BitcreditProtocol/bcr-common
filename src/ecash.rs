// ----- standard library imports
// ----- extra library imports
use bitcoin::bip32 as btc32;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;
// ----- local imports
use crate::wire;

// ----- end imports

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("dhke {0}")]
    Dhke(#[from] cashu::dhke::Error),
    #[error("amount {0}")]
    Amount(#[from] cashu::amount::Error),
}

#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, BorshSerialize, BorshDeserialize,
)]
pub struct BlindedMessage {
    #[borsh(
        serialize_with = "wire::borsh::serialize_as_u64",
        deserialize_with = "wire::borsh::deserialize_from_u64"
    )]
    pub amount: cashu::Amount,
    #[serde(rename = "id")]
    #[borsh(
        serialize_with = "wire::borsh::serialize_as_str",
        deserialize_with = "wire::borsh::deserialize_from_str"
    )]
    pub keyset_id: cashu::Id,
    #[serde(rename = "B_")]
    #[borsh(
        serialize_with = "wire::borsh::serialize_as_str",
        deserialize_with = "wire::borsh::deserialize_from_str"
    )]
    pub blinded_secret: cashu::PublicKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[borsh(
        serialize_with = "wire::borsh::serialize_optionproofwitness",
        deserialize_with = "wire::borsh::deserialize_optionproofwitness"
    )]
    pub witness: Option<cashu::Witness>,
}

#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, BorshSerialize, BorshDeserialize,
)]
pub struct BlindSignature {
    #[borsh(
        serialize_with = "wire::borsh::serialize_as_u64",
        deserialize_with = "wire::borsh::deserialize_from_u64"
    )]
    pub amount: cashu::Amount,
    #[serde(rename = "id")]
    #[borsh(
        serialize_with = "wire::borsh::serialize_as_str",
        deserialize_with = "wire::borsh::deserialize_from_str"
    )]
    pub keyset_id: cashu::Id,
    #[serde(rename = "C_")]
    #[borsh(
        serialize_with = "wire::borsh::serialize_as_str",
        deserialize_with = "wire::borsh::deserialize_from_str"
    )]
    pub c: cashu::PublicKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[borsh(
        serialize_with = "wire::borsh::serialize_option_blindsigdleq",
        deserialize_with = "wire::borsh::deserialize_option_blindsigdleq"
    )]
    pub dleq: Option<cashu::BlindSignatureDleq>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Proof {
    pub amount: cashu::Amount,
    #[serde(rename = "id")]
    pub keyset_id: cashu::Id,
    #[schema(value_type = String)]
    pub secret: cashu::secret::Secret,
    #[serde(rename = "C")]
    pub c: cashu::PublicKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub witness: Option<cashu::Witness>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dleq: Option<cashu::ProofDleq>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p2pk_e: Option<cashu::PublicKey>,
}

pub type Proofs = Vec<Proof>;

/// `y`, the `hash_to_curve(secret)` point the mint identifies a proof by
pub fn y_of(secret: &cashu::secret::Secret) -> Result<cashu::PublicKey> {
    Ok(cashu::dhke::hash_to_curve(secret.as_bytes())?)
}

impl Proof {
    /// `y`, the `hash_to_curve(secret)` point the mint uses to identify this proof
    pub fn y(&self) -> Result<cashu::PublicKey> {
        y_of(&self.secret)
    }
}

/// Aggregates over a collection of [`Proof`]s. Implemented for `[Proof]`, so it
/// reaches [`Proofs`] and any slice of proofs alike
pub trait ProofsMethods {
    /// Total value of the proofs
    fn total_amount(&self) -> Result<cashu::Amount>;

    /// `y` of every proof, in order
    fn ys(&self) -> Result<Vec<cashu::PublicKey>>;
}

impl ProofsMethods for [Proof] {
    fn total_amount(&self) -> Result<cashu::Amount> {
        Ok(cashu::Amount::try_sum(self.iter().map(|p| p.amount))?)
    }

    fn ys(&self) -> Result<Vec<cashu::PublicKey>> {
        self.iter().map(Proof::y).collect()
    }
}

impl From<cashu::Proof> for Proof {
    fn from(proof: cashu::Proof) -> Self {
        Self {
            amount: proof.amount,
            keyset_id: proof.keyset_id,
            secret: proof.secret,
            c: proof.c,
            witness: proof.witness,
            dleq: proof.dleq,
            p2pk_e: proof.p2pk_e,
        }
    }
}

impl From<Proof> for cashu::Proof {
    fn from(proof: Proof) -> Self {
        Self {
            amount: proof.amount,
            keyset_id: proof.keyset_id,
            secret: proof.secret,
            c: proof.c,
            witness: proof.witness,
            dleq: proof.dleq,
            p2pk_e: proof.p2pk_e,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct KeySet {
    pub id: cashu::Id,
    pub unit: cashu::CurrencyUnit,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    pub keys: cashu::Keys,
    #[serde(default)]
    pub input_fee_ppk: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_expiry: Option<u64>,
}

impl From<cashu::KeySet> for KeySet {
    fn from(keyset: cashu::KeySet) -> Self {
        Self {
            id: keyset.id,
            unit: keyset.unit,
            active: keyset.active,
            keys: keyset.keys,
            input_fee_ppk: keyset.input_fee_ppk,
            final_expiry: keyset.final_expiry,
        }
    }
}

impl From<KeySet> for cashu::KeySet {
    fn from(keyset: KeySet) -> Self {
        Self {
            id: keyset.id,
            unit: keyset.unit,
            active: keyset.active,
            keys: keyset.keys,
            input_fee_ppk: keyset.input_fee_ppk,
            final_expiry: keyset.final_expiry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct KeySetInfo {
    pub id: cashu::Id,
    pub unit: cashu::CurrencyUnit,
    pub active: bool,
    pub input_fee_ppk: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_expiry: Option<u64>,
}

impl From<cashu::KeySetInfo> for KeySetInfo {
    fn from(info: cashu::KeySetInfo) -> Self {
        Self {
            id: info.id,
            unit: info.unit,
            active: info.active,
            input_fee_ppk: info.input_fee_ppk,
            final_expiry: info.final_expiry,
        }
    }
}

impl From<KeySetInfo> for cashu::KeySetInfo {
    fn from(info: KeySetInfo) -> Self {
        Self {
            id: info.id,
            unit: info.unit,
            active: info.active,
            input_fee_ppk: info.input_fee_ppk,
            final_expiry: info.final_expiry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MintKeySet {
    pub id: cashu::Id,
    pub unit: cashu::CurrencyUnit,
    pub keys: cashu::nut01::MintKeys,
    pub input_fee_ppk: u64,
    pub final_expiry: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MintKeySetInfo {
    pub id: cashu::Id,
    pub unit: cashu::CurrencyUnit,
    pub active: bool,
    pub valid_from: u64,
    pub derivation_path: btc32::DerivationPath,
    pub derivation_path_index: Option<u32>,
    pub amounts: Vec<u64>,
    pub input_fee_ppk: u64,
    pub final_expiry: Option<u64>,
}

impl From<cashu::MintKeySet> for MintKeySet {
    fn from(keyset: cashu::MintKeySet) -> Self {
        Self {
            id: keyset.id,
            unit: keyset.unit,
            keys: keyset.keys,
            input_fee_ppk: keyset.input_fee_ppk,
            final_expiry: keyset.final_expiry,
        }
    }
}

impl From<MintKeySet> for cashu::MintKeySet {
    fn from(keyset: MintKeySet) -> Self {
        Self {
            id: keyset.id,
            unit: keyset.unit,
            keys: keyset.keys,
            input_fee_ppk: keyset.input_fee_ppk,
            final_expiry: keyset.final_expiry,
        }
    }
}

impl From<MintKeySetInfo> for KeySetInfo {
    fn from(info: MintKeySetInfo) -> Self {
        Self {
            id: info.id,
            unit: info.unit,
            active: info.active,
            input_fee_ppk: info.input_fee_ppk,
            final_expiry: info.final_expiry,
        }
    }
}

impl From<MintKeySet> for KeySet {
    fn from(keyset: MintKeySet) -> Self {
        Self {
            id: keyset.id,
            unit: keyset.unit,
            active: None,
            keys: keyset.keys.into(),
            input_fee_ppk: keyset.input_fee_ppk,
            final_expiry: keyset.final_expiry,
        }
    }
}

impl From<cdk_common::mint::MintKeySetInfo> for MintKeySetInfo {
    fn from(info: cdk_common::mint::MintKeySetInfo) -> Self {
        Self {
            id: info.id,
            unit: info.unit,
            active: info.active,
            valid_from: info.valid_from,
            derivation_path: info.derivation_path,
            derivation_path_index: info.derivation_path_index,
            amounts: info.amounts,
            input_fee_ppk: info.input_fee_ppk,
            final_expiry: info.final_expiry,
        }
    }
}

impl From<MintKeySetInfo> for cashu::KeySetInfo {
    fn from(info: MintKeySetInfo) -> Self {
        Self {
            id: info.id,
            unit: info.unit,
            active: info.active,
            input_fee_ppk: info.input_fee_ppk,
            final_expiry: info.final_expiry,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core, core_tests};

    fn random_keyset_id() -> cashu::Id {
        let mut bytes = [0; 8];
        bytes[1..].copy_from_slice(&rand::random::<[u8; 7]>());
        cashu::Id::from_bytes(&bytes).expect("keyset id")
    }

    fn random_public_key() -> cashu::PublicKey {
        cashu::PublicKey::from(core::generate_random_keypair().public_key())
    }

    fn random_mint_keyset() -> MintKeySet {
        let (_, keyset) = core_tests::generate_random_ecash_keyset();
        keyset
    }

    #[test]
    fn blindedmessage_json_wire_compat() {
        let secret = cashu::secret::Secret::new(rand::random::<u64>().to_string());
        let (blinded_secret, _) =
            cashu::dhke::blind_message(secret.as_bytes(), None).expect("blind message");
        let message = BlindedMessage {
            amount: cashu::Amount::from(rand::random::<u16>() as u64),
            keyset_id: random_keyset_id(),
            blinded_secret,
            witness: None,
        };
        let bytes = serde_json::to_vec(&message).expect("serialize");
        let deserialized: cashu::BlindedMessage =
            serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized.amount, message.amount);
        assert_eq!(deserialized.keyset_id, message.keyset_id);
        assert_eq!(deserialized.blinded_secret, message.blinded_secret);
        assert_eq!(deserialized.witness, message.witness);
    }

    #[test]
    fn blindsignature_json_wire_compat() {
        let signature = BlindSignature {
            amount: cashu::Amount::from(rand::random::<u16>() as u64),
            keyset_id: random_keyset_id(),
            c: random_public_key(),
            dleq: None,
        };
        let bytes = serde_json::to_vec(&signature).expect("serialize");
        let deserialized: cashu::BlindSignature =
            serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized.amount, signature.amount);
        assert_eq!(deserialized.keyset_id, signature.keyset_id);
        assert_eq!(deserialized.c, signature.c);
        assert_eq!(deserialized.dleq, signature.dleq);
    }

    #[test]
    fn proof_json_wire_compat() {
        let keyset = random_mint_keyset();
        let cashu_proof =
            core_tests::generate_random_ecash_proofs(&keyset, &[cashu::Amount::from(1u64)])
                .remove(0);
        let proof = Proof {
            amount: cashu_proof.amount,
            keyset_id: cashu_proof.keyset_id,
            secret: cashu_proof.secret,
            c: cashu_proof.c,
            witness: cashu_proof.witness,
            dleq: cashu_proof.dleq,
            p2pk_e: cashu_proof.p2pk_e,
        };
        let bytes = serde_json::to_vec(&proof).expect("serialize");
        let deserialized: cashu::Proof = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized.secret, proof.secret);
    }

    #[test]
    fn keyset_json_wire_compat() {
        let mint_keyset = random_mint_keyset().into();
        let cashu_keyset = core::keys::to_keyset(&mint_keyset, Some(true));
        let keyset = KeySet {
            id: cashu_keyset.id,
            unit: cashu_keyset.unit,
            active: cashu_keyset.active,
            keys: cashu_keyset.keys,
            input_fee_ppk: cashu_keyset.input_fee_ppk,
            final_expiry: cashu_keyset.final_expiry,
        };
        let bytes = serde_json::to_vec(&keyset).expect("serialize");
        let deserialized: cashu::KeySet = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized.id, keyset.id);
    }

    #[test]
    fn mintkeyset_json_wire_compat() {
        let cashu_mint_keyset = random_mint_keyset();
        let mint_keyset = MintKeySet {
            id: cashu_mint_keyset.id,
            unit: cashu_mint_keyset.unit,
            keys: cashu_mint_keyset.keys,
            input_fee_ppk: cashu_mint_keyset.input_fee_ppk,
            final_expiry: cashu_mint_keyset.final_expiry,
        };
        let bytes = serde_json::to_vec(&mint_keyset).expect("serialize");
        let deserialized: cashu::MintKeySet = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized.keys, mint_keyset.keys);
    }

    /// `y` and `total_amount` must agree with cashu's, since the mint is the one
    /// checking both
    #[test]
    fn proof_utilities_match_cashu() {
        let keyset = random_mint_keyset();
        let cashu_proofs = core_tests::generate_random_ecash_proofs(
            &keyset,
            &[cashu::Amount::from(1u64), cashu::Amount::from(8u64)],
        );
        let proofs: Proofs = cashu_proofs.iter().cloned().map(Into::into).collect();

        for (proof, expected) in proofs.iter().zip(cashu_proofs.iter()) {
            assert_eq!(proof.y().unwrap(), expected.y().unwrap());
        }
        assert_eq!(proofs.ys().unwrap().len(), 2);
        assert_eq!(proofs.total_amount().unwrap(), cashu::Amount::from(9u64));
        assert_eq!(
            proofs.total_amount().unwrap(),
            cashu::nut00::ProofsMethods::total_amount(&cashu_proofs).unwrap()
        );
        assert_eq!(Proofs::new().total_amount().unwrap(), cashu::Amount::ZERO);
    }

    #[test]
    fn cashu_conversions_round_trip() {
        let keyset = random_mint_keyset();
        let proof: Proof =
            core_tests::generate_random_ecash_proofs(&keyset, &[cashu::Amount::from(1u64)])
                .remove(0)
                .into();
        assert_eq!(Proof::from(cashu::Proof::from(proof.clone())), proof);

        let info = KeySetInfo {
            id: keyset.id,
            unit: keyset.unit,
            active: true,
            input_fee_ppk: keyset.input_fee_ppk,
            final_expiry: keyset.final_expiry,
        };
        assert_eq!(
            KeySetInfo::from(cashu::KeySetInfo::from(info.clone())),
            info
        );
    }

    #[test]
    fn keyset_info_json_wire_compat() {
        let (_, mint_keyset) = core_tests::generate_random_ecash_keyset();
        let keyset_info = KeySetInfo {
            id: mint_keyset.id,
            unit: mint_keyset.unit,
            active: true,
            input_fee_ppk: mint_keyset.input_fee_ppk,
            final_expiry: mint_keyset.final_expiry,
        };
        let bytes = serde_json::to_vec(&keyset_info).expect("serialize");
        let deserialized: cashu::KeySetInfo = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized.id, keyset_info.id);
    }
}
