// ----- standard library imports
use std::str::FromStr;
// ----- extra library imports
use bitcoin::{
    bip32 as btc32,
    hex::{DisplayHex, FromHex},
    secp256k1 as secp,
};
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
    #[error("invalid keyset id")]
    InvalidKeysetId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Id {
    V1([u8; 7]),
    V2([u8; 32]),
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1(body) => write!(f, "00{}", body.as_hex()),
            Self::V2(body) => write!(f, "01{}", body.as_hex()),
        }
    }
}

impl FromStr for Id {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.split_at_checked(2) {
            Some(("00", body)) => Ok(Self::V1(
                FromHex::from_hex(body).map_err(|_| Error::InvalidKeysetId)?,
            )),
            Some(("01", body)) => Ok(Self::V2(
                FromHex::from_hex(body).map_err(|_| Error::InvalidKeysetId)?,
            )),
            _ => Err(Error::InvalidKeysetId),
        }
    }
}

impl serde::Serialize for Id {
    fn serialize<S>(&self, s: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        s.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for Id {
    fn deserialize<D>(d: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <std::string::String as serde::Deserialize>::deserialize(d)?;
        Id::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl BorshSerialize for Id {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let id_str = self.to_string();
        BorshSerialize::serialize(&id_str, writer)
    }
}

impl BorshDeserialize for Id {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let id_str: String = BorshDeserialize::deserialize_reader(reader)?;
        Id::from_str(&id_str).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

impl From<cashu::Id> for Id {
    fn from(id: cashu::Id) -> Self {
        Id::from_str(&id.to_string()).expect("cashu::Id renders a valid keyset id")
    }
}

impl From<Id> for cashu::Id {
    fn from(id: Id) -> Self {
        cashu::Id::from_str(&id.to_string()).expect("ecash::Id renders a valid keyset id")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct BlindedMessage {
    #[borsh(
        serialize_with = "wire::borsh::serialize_btc_amount",
        deserialize_with = "wire::borsh::deserialize_btc_amount"
    )]
    pub amount: bitcoin::Amount,
    #[serde(rename = "id")]
    pub keyset_id: Id,
    #[serde(rename = "B_")]
    #[borsh(
        serialize_with = "wire::borsh::serialize_as_str",
        deserialize_with = "wire::borsh::deserialize_from_str"
    )]
    pub blinded_secret: secp::PublicKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[borsh(
        serialize_with = "wire::borsh::serialize_optionproofwitness",
        deserialize_with = "wire::borsh::deserialize_optionproofwitness"
    )]
    pub witness: Option<cashu::Witness>,
}

impl From<cashu::BlindedMessage> for BlindedMessage {
    fn from(message: cashu::BlindedMessage) -> Self {
        let blinded_secret = public_key_cashu2secp(&message.blinded_secret);
        Self {
            amount: bitcoin::Amount::from_sat(message.amount.into()),
            keyset_id: message.keyset_id.into(),
            blinded_secret,
            witness: message.witness,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct BlindSignature {
    #[borsh(
        serialize_with = "wire::borsh::serialize_btc_amount",
        deserialize_with = "wire::borsh::deserialize_btc_amount"
    )]
    pub amount: bitcoin::Amount,
    #[serde(rename = "id")]
    pub keyset_id: Id,
    #[serde(rename = "C_")]
    #[borsh(
        serialize_with = "wire::borsh::serialize_as_str",
        deserialize_with = "wire::borsh::deserialize_from_str"
    )]
    pub c: secp::PublicKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[borsh(
        serialize_with = "wire::borsh::serialize_option_blindsigdleq",
        deserialize_with = "wire::borsh::deserialize_option_blindsigdleq"
    )]
    pub dleq: Option<cashu::BlindSignatureDleq>,
}

impl From<BlindSignature> for cashu::BlindSignature {
    fn from(signature: BlindSignature) -> Self {
        Self {
            amount: cashu::Amount::from(signature.amount.to_sat()),
            keyset_id: signature.keyset_id.into(),
            c: public_key_secp2cashu(&signature.c),
            dleq: signature.dleq,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proof {
    pub amount: bitcoin::Amount,
    #[serde(rename = "id")]
    pub keyset_id: Id,
    pub secret: cashu::secret::Secret,
    #[serde(rename = "C")]
    pub c: secp::PublicKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub witness: Option<cashu::Witness>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dleq: Option<cashu::ProofDleq>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p2pk_e: Option<secp::PublicKey>,
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
        let btc_amount: bitcoin::Amount = self.iter().map(|p| p.amount).sum();
        Ok(cashu::Amount::from(btc_amount.to_sat()))
    }

    fn ys(&self) -> Result<Vec<cashu::PublicKey>> {
        self.iter().map(Proof::y).collect()
    }
}

impl From<cashu::Proof> for Proof {
    fn from(proof: cashu::Proof) -> Self {
        Self {
            amount: bitcoin::Amount::from_sat(proof.amount.into()),
            keyset_id: proof.keyset_id.into(),
            secret: (proof.secret),
            c: public_key_cashu2secp(&proof.c),
            witness: proof.witness,
            dleq: proof.dleq,
            p2pk_e: proof.p2pk_e.as_ref().map(public_key_cashu2secp),
        }
    }
}

impl From<Proof> for cashu::Proof {
    fn from(proof: Proof) -> Self {
        Self {
            amount: cashu::Amount::from(proof.amount.to_sat()),
            keyset_id: proof.keyset_id.into(),
            secret: proof.secret,
            c: public_key_secp2cashu(&proof.c),
            witness: proof.witness,
            dleq: proof.dleq,
            p2pk_e: proof.p2pk_e.as_ref().map(public_key_secp2cashu),
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
    #[schema(value_type = String)]
    pub id: Id,
    pub unit: cashu::CurrencyUnit,
    pub active: bool,
    pub input_fee_ppk: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_expiry: Option<u64>,
}

impl From<cashu::KeySetInfo> for KeySetInfo {
    fn from(info: cashu::KeySetInfo) -> Self {
        Self {
            id: info.id.into(),
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
            id: info.id.into(),
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
            id: info.id.into(),
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

// TODO: use from_byte_array_compressed when on secp256k1@0.30
pub(crate) fn public_key_secp2cashu(pk: &secp::PublicKey) -> cashu::PublicKey {
    cashu::PublicKey::from_slice(&pk.serialize())
        .expect("secp256k1::PublicKey <-> cashu::PublicKey")
}

pub(crate) fn public_key_cashu2secp(pk: &cashu::PublicKey) -> secp::PublicKey {
    secp::PublicKey::from_slice(&pk.to_bytes()).expect("cashu::PublicKey <-> secp256k1::PublicKey")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core, core_tests};

    fn id_fixtures() -> Vec<String> {
        vec![
            String::from("00ad268c4d1f5826"),
            String::from("000f01df73ea149a"),
            String::from("00ffffffffffffff"),
            format!("01{}", "00".repeat(32)),
            format!("01aabbccddeeff00{}", "11".repeat(25)),
            format!("01aabbccddeeff00{}", "22".repeat(25)),
        ]
    }

    #[test]
    fn id_json_wire_compat() {
        for s in id_fixtures() {
            let id = Id::from_str(&s).expect("parse");
            assert_eq!(id.to_string(), s);

            let bytes = serde_json::to_vec(&id).expect("serialize");
            let deserialized: cashu::Id = serde_json::from_slice(&bytes).expect("deserialize");
            assert_eq!(deserialized, cashu::Id::from(id));
            assert_eq!(bytes, serde_json::to_vec(&deserialized).expect("serialize"));
            assert_eq!(bytes, format!("\"{s}\"").into_bytes());

            let back: Id = serde_json::from_slice(&bytes).expect("deserialize");
            assert_eq!(back, id);
            assert_eq!(Id::from(cashu::Id::from(id)), id);
        }
    }

    #[test]
    fn id_borsh_wire_compat() {
        for s in id_fixtures() {
            let id = Id::from_str(&s).expect("parse");
            let kid = cashu::Id::from(id);

            let mut buf = Vec::new();
            BorshSerialize::serialize(&id, &mut buf).expect("serialize");
            let mut upstream = Vec::new();
            wire::borsh::serialize_as_str(&kid, &mut upstream).expect("serialize");
            assert_eq!(buf, upstream);
            assert_eq!(buf[..4], (s.len() as u32).to_le_bytes());

            let deserialized: Id = BorshDeserialize::deserialize_reader(&mut upstream.as_slice())
                .expect("deserialize");
            assert_eq!(deserialized, id);
            let upstream_back: cashu::Id =
                wire::borsh::deserialize_from_str(&mut buf.as_slice()).expect("deserialize");
            assert_eq!(upstream_back, kid);
        }
    }

    #[test]
    fn id_ord_matches_cashu() {
        let strs = id_fixtures();
        let mut ids: Vec<Id> = strs
            .iter()
            .map(|s| Id::from_str(s).expect("parse"))
            .collect();
        let mut kids: Vec<cashu::Id> = strs
            .iter()
            .map(|s| cashu::Id::from_str(s).expect("parse"))
            .collect();
        ids.sort();
        kids.sort();
        assert_eq!(
            ids.into_iter().map(cashu::Id::from).collect::<Vec<_>>(),
            kids
        );
    }

    #[test]
    fn id_accept_reject_matches_cashu() {
        let refused = vec![
            String::new(),
            String::from("0"),
            String::from("00"),
            String::from("00ad268c4d1f582"),
            String::from("00ad268c4d1f58267"),
            format!("01{}", "0".repeat(63)),
            format!("01{}", "0".repeat(65)),
            String::from("02ad268c4d1f5826"),
            String::from("ffad268c4d1f5826"),
            format!("00{}", "11".repeat(32)),
            String::from("01ad268c4d1f5826"),
            String::from("0gad268c4d1f5826"),
            String::from("00ad268c4d1f582g"),
            format!("0é{}", "0".repeat(13)),
        ];
        for s in refused {
            assert!(Id::from_str(&s).is_err(), "should refuse {s:?}");
            assert!(cashu::Id::from_str(&s).is_err(), "cashu refuses {s:?} too");
        }

        let id = Id::from_str("00AD268C4D1F5826").expect("parse");
        assert_eq!(id.to_string(), "00ad268c4d1f5826");
        assert_eq!(
            id,
            Id::from(cashu::Id::from_str("00AD268C4D1F5826").expect("parse"))
        );
    }

    fn random_keyset_id() -> Id {
        let mut bytes = [0; 8];
        bytes[1..].copy_from_slice(&rand::random::<[u8; 7]>());
        cashu::Id::from_bytes(&bytes).expect("keyset id").into()
    }

    #[test]
    fn blindedmessage_json_wire_compat() {
        let secret = cashu::secret::Secret::new(rand::random::<u64>().to_string());
        let (blinded_secret, _) =
            cashu::dhke::blind_message(secret.as_bytes(), None).expect("blind message");
        let message = BlindedMessage {
            amount: bitcoin::Amount::from_sat(rand::random::<u16>() as u64),
            keyset_id: random_keyset_id(),
            blinded_secret: public_key_cashu2secp(&blinded_secret),
            witness: None,
        };
        let bytes = serde_json::to_vec(&message).expect("serialize");
        let deserialized: cashu::BlindedMessage =
            serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(u64::from(deserialized.amount), message.amount.to_sat());
        assert_eq!(Id::from(deserialized.keyset_id), message.keyset_id);
        assert_eq!(
            public_key_cashu2secp(&deserialized.blinded_secret),
            message.blinded_secret
        );
        assert_eq!(deserialized.witness, message.witness);
    }

    #[test]
    fn blindsignature_json_wire_compat() {
        let signature = BlindSignature {
            amount: bitcoin::Amount::from_sat(rand::random::<u16>() as u64),
            keyset_id: random_keyset_id(),
            c: core::generate_random_keypair().public_key(),
            dleq: None,
        };
        let bytes = serde_json::to_vec(&signature).expect("serialize");
        let deserialized: cashu::BlindSignature =
            serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(u64::from(deserialized.amount), signature.amount.to_sat());
        assert_eq!(Id::from(deserialized.keyset_id), signature.keyset_id);
        assert_eq!(public_key_cashu2secp(&deserialized.c), signature.c);
        assert_eq!(deserialized.dleq, signature.dleq);
    }

    #[test]
    fn proof_json_wire_compat() {
        let keyset = core_tests::generate_random_ecash_keyset().1;
        let cashu_proof =
            core_tests::generate_random_ecash_proofs(&keyset, &[cashu::Amount::from(1u64)])
                .remove(0);
        let proof = Proof {
            amount: bitcoin::Amount::from_sat(cashu_proof.amount.into()),
            keyset_id: cashu_proof.keyset_id.into(),
            secret: cashu_proof.secret,
            c: public_key_cashu2secp(&cashu_proof.c),
            witness: cashu_proof.witness,
            dleq: cashu_proof.dleq,
            p2pk_e: cashu_proof.p2pk_e.as_ref().map(public_key_cashu2secp),
        };
        let bytes = serde_json::to_vec(&proof).expect("serialize");
        let deserialized: cashu::Proof = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized.secret, proof.secret);
    }

    #[test]
    fn keyset_json_wire_compat() {
        let mint_keyset = core_tests::generate_random_ecash_keyset().1;
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
        let cashu_mint_keyset = core_tests::generate_random_ecash_keyset().1;
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
        let keyset = core_tests::generate_random_ecash_keyset().1;
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
        let keyset = core_tests::generate_random_ecash_keyset().1;
        let proof: Proof =
            core_tests::generate_random_ecash_proofs(&keyset, &[cashu::Amount::from(1u64)])
                .remove(0)
                .into();
        assert_eq!(Proof::from(cashu::Proof::from(proof.clone())), proof);
        let info = KeySetInfo {
            id: keyset.id.into(),
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
            id: mint_keyset.id.into(),
            unit: mint_keyset.unit,
            active: true,
            input_fee_ppk: mint_keyset.input_fee_ppk,
            final_expiry: mint_keyset.final_expiry,
        };
        let bytes = serde_json::to_vec(&keyset_info).expect("serialize");
        let deserialized: cashu::KeySetInfo = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(Id::from(deserialized.id), keyset_info.id);
    }
}
