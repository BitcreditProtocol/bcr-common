// ----- standard library imports
use std::{
    convert::{From, TryFrom},
    str::FromStr,
};
// ----- extra library imports
use bitcoin::secp256k1;
use borsh::io::{Error as BorshError, ErrorKind, Read, Write};
// ----- local imports

// ----- end imports

type Result<T> = core::result::Result<T, BorshError>;

pub fn serialize_as_str<T>(t: &T, writer: &mut impl Write) -> Result<()>
where
    T: std::fmt::Display,
{
    let stringified = t.to_string();
    borsh::BorshSerialize::serialize(&stringified, writer)?;
    Ok(())
}
pub fn deserialize_from_str<T>(reader: &mut impl Read) -> Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    let stringified: String = borsh::BorshDeserialize::deserialize_reader(reader)?;
    let t = T::from_str(&stringified).map_err(|e| BorshError::new(ErrorKind::InvalidInput, e))?;
    Ok(t)
}

pub fn serialize_as_u64<T>(t: &T, writer: &mut impl Write) -> Result<()>
where
    T: Clone,
    u64: From<T>,
{
    let value: u64 = u64::from(t.clone());
    borsh::BorshSerialize::serialize(&value, writer)?;
    Ok(())
}

pub fn deserialize_from_u64<T>(reader: &mut impl Read) -> Result<T>
where
    T: TryFrom<u64>,
    <T as TryFrom<u64>>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    let value: u64 = borsh::BorshDeserialize::deserialize_reader(reader)?;
    let t = T::try_from(value).map_err(|e| BorshError::new(ErrorKind::InvalidInput, e))?;
    Ok(t)
}

pub fn serialize_vec_of_strs<T>(vec: &[T], writer: &mut impl Write) -> Result<()>
where
    T: std::fmt::Display,
{
    let strs: Vec<String> = vec.iter().map(|v| v.to_string()).collect();
    borsh::BorshSerialize::serialize(&strs, writer)?;
    Ok(())
}

pub fn deserialize_vec_of_strs<T>(reader: &mut impl Read) -> Result<Vec<T>>
where
    T: std::str::FromStr,
    <T as FromStr>::Err: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    let strs: Vec<String> = borsh::BorshDeserialize::deserialize_reader(reader)?;
    strs.into_iter()
        .map(|v| T::from_str(&v))
        .collect::<std::result::Result<Vec<T>, T::Err>>()
        .map_err(|e| BorshError::new(ErrorKind::InvalidData, e))
}
pub(crate) fn serialize_vec_of_jsons<T>(vec: &[T], writer: &mut impl Write) -> Result<()>
where
    T: serde::ser::Serialize,
{
    let stringified =
        serde_json::to_string(vec).map_err(|e| BorshError::new(ErrorKind::InvalidInput, e))?;
    borsh::BorshSerialize::serialize(&stringified, writer)?;
    Ok(())
}

pub(crate) fn deserialize_vec_of_jsons<T>(reader: &mut impl Read) -> Result<Vec<T>>
where
    T: serde::de::DeserializeOwned,
{
    let stringified: String = borsh::BorshDeserialize::deserialize_reader(reader)?;
    let vec = serde_json::from_str(&stringified)
        .map_err(|e| BorshError::new(ErrorKind::InvalidData, e))?;
    Ok(vec)
}

#[derive(Debug, Clone, borsh::BorshSerialize, borsh::BorshDeserialize)]
struct Dleq {
    e: String,
    s: String,
    r: String,
}
impl std::convert::From<cashu::ProofDleq> for Dleq {
    fn from(witness: cashu::ProofDleq) -> Self {
        Dleq {
            e: witness.e.to_string(),
            s: witness.s.to_string(),
            r: witness.r.to_string(),
        }
    }
}
impl std::convert::TryFrom<Dleq> for cashu::ProofDleq {
    type Error = BorshError;
    fn try_from(dleq: Dleq) -> Result<Self> {
        let e = cashu::SecretKey::from_str(&dleq.e)
            .map_err(|e| BorshError::new(ErrorKind::InvalidData, e))?;
        let s = cashu::SecretKey::from_str(&dleq.s)
            .map_err(|e| BorshError::new(ErrorKind::InvalidData, e))?;
        let r = cashu::SecretKey::from_str(&dleq.r)
            .map_err(|e| BorshError::new(ErrorKind::InvalidData, e))?;
        Ok(cashu::ProofDleq { e, s, r })
    }
}
pub fn serialize_optionproofdleq(
    dleq: &Option<cashu::ProofDleq>,
    writer: &mut impl Write,
) -> Result<()> {
    let dleq = dleq.clone().map(Dleq::from);
    borsh::BorshSerialize::serialize(&dleq, writer)?;
    Ok(())
}

pub fn deserialize_optionproofdleq(reader: &mut impl Read) -> Result<Option<cashu::ProofDleq>> {
    let dleq: Option<Dleq> = borsh::BorshDeserialize::deserialize_reader(reader)?;
    let dleq = dleq.map(cashu::ProofDleq::try_from).transpose()?;
    Ok(dleq)
}

#[derive(Debug, Clone, borsh::BorshSerialize, borsh::BorshDeserialize)]
enum WitnessEnum {
    HTLCWitness {
        preimage: String,
        signatures: Option<Vec<String>>,
    },
    P2PKWitness {
        signatures: Vec<String>,
    },
}
impl std::convert::From<cashu::Witness> for WitnessEnum {
    fn from(witness: cashu::Witness) -> Self {
        match witness {
            cashu::Witness::HTLCWitness(htlc) => WitnessEnum::HTLCWitness {
                preimage: htlc.preimage,
                signatures: htlc.signatures,
            },
            cashu::Witness::P2PKWitness(p2pk) => WitnessEnum::P2PKWitness {
                signatures: p2pk.signatures,
            },
        }
    }
}
impl std::convert::From<WitnessEnum> for cashu::Witness {
    fn from(witness_enum: WitnessEnum) -> Self {
        match witness_enum {
            WitnessEnum::HTLCWitness {
                preimage,
                signatures,
            } => cashu::Witness::HTLCWitness(cashu::HTLCWitness {
                preimage,
                signatures,
            }),
            WitnessEnum::P2PKWitness { signatures } => {
                cashu::Witness::P2PKWitness(cashu::P2PKWitness { signatures })
            }
        }
    }
}
pub fn serialize_optionwitness(
    witness: &Option<cashu::Witness>,
    writer: &mut impl Write,
) -> Result<()> {
    let enumed = witness.as_ref().map(|w| WitnessEnum::from(w.clone()));
    borsh::BorshSerialize::serialize(&enumed, writer)?;
    Ok(())
}

pub fn deserialize_optionwitness(reader: &mut impl Read) -> Result<Option<cashu::Witness>> {
    let enumed: Option<WitnessEnum> = borsh::BorshDeserialize::deserialize_reader(reader)?;
    let witness = enumed.map(cashu::Witness::from);
    Ok(witness)
}

#[derive(Debug, Clone, borsh::BorshSerialize, borsh::BorshDeserialize)]
struct Proof {
    amount: u64,
    kid: Vec<u8>,
    secret: String,
    c: [u8; secp256k1::constants::PUBLIC_KEY_SIZE],
    dleq: Option<Dleq>,
    witness: Option<WitnessEnum>,
}
impl std::convert::From<cashu::Proof> for Proof {
    fn from(proof: cashu::Proof) -> Self {
        Proof {
            amount: u64::from(proof.amount),
            kid: proof.keyset_id.to_bytes(),
            secret: proof.secret.to_string(),
            c: proof.c.to_bytes(),
            dleq: proof.dleq.map(Dleq::from),
            witness: proof.witness.map(WitnessEnum::from),
        }
    }
}
impl std::convert::From<Proof> for cashu::Proof {
    fn from(proof: Proof) -> Self {
        let keyset_id = cashu::Id::from_bytes(&proof.kid).expect("keyset_id parse");
        let secret = cashu::secret::Secret::from_str(&proof.secret).expect("secret parse");
        let dleq = proof
            .dleq
            .map(cashu::ProofDleq::try_from)
            .transpose()
            .expect("dleq parse");
        let c = cashu::PublicKey::from_slice(&proof.c).expect("c parse");
        cashu::Proof {
            amount: cashu::Amount::from(proof.amount),
            keyset_id,
            c,
            secret,
            dleq,
            witness: proof.witness.map(cashu::Witness::from),
        }
    }
}
pub fn serialize_cdkproof(input: &cashu::Proof, writer: &mut impl Write) -> Result<()> {
    let proof = Proof::from(input.clone());
    borsh::BorshSerialize::serialize(&proof, writer)?;
    Ok(())
}
pub fn serialize_vecof_cdkproof(input: &[cashu::Proof], writer: &mut impl Write) -> Result<()> {
    let proofs: Vec<_> = input.iter().cloned().map(Proof::from).collect();
    borsh::BorshSerialize::serialize(&proofs, writer)?;
    Ok(())
}

pub fn deserialize_cdkproof(reader: &mut impl Read) -> Result<cashu::Proof> {
    let proof: Proof = borsh::BorshDeserialize::deserialize_reader(reader)?;
    let output = cashu::Proof::from(proof);
    Ok(output)
}
pub fn deserialize_vecof_cdkproof(reader: &mut impl Read) -> Result<Vec<cashu::Proof>> {
    let proofs: Vec<Proof> = borsh::BorshDeserialize::deserialize_reader(reader)?;
    let output: Vec<cashu::Proof> = proofs.into_iter().map(cashu::Proof::from).collect();
    Ok(output)
}

#[derive(Debug, Clone, borsh::BorshSerialize, borsh::BorshDeserialize)]
struct BlindedMessageBorsh {
    amount: u64,
    kid: Vec<u8>,
    blinded_secret: [u8; secp256k1::constants::PUBLIC_KEY_SIZE],
    witness: Option<WitnessEnum>,
}
impl std::convert::From<cashu::BlindedMessage> for BlindedMessageBorsh {
    fn from(msg: cashu::BlindedMessage) -> Self {
        BlindedMessageBorsh {
            amount: u64::from(msg.amount),
            kid: msg.keyset_id.to_bytes(),
            blinded_secret: msg.blinded_secret.to_bytes(),
            witness: msg.witness.map(WitnessEnum::from),
        }
    }
}
impl std::convert::From<BlindedMessageBorsh> for cashu::BlindedMessage {
    fn from(msg: BlindedMessageBorsh) -> Self {
        cashu::BlindedMessage {
            amount: cashu::Amount::from(msg.amount),
            keyset_id: cashu::Id::from_bytes(&msg.kid).expect("keyset_id parse"),
            blinded_secret: cashu::PublicKey::from_slice(&msg.blinded_secret)
                .expect("blinded_secret parse"),
            witness: msg.witness.map(cashu::Witness::from),
        }
    }
}
pub fn serialize_vecof_blindedmessage(
    input: &[cashu::BlindedMessage],
    writer: &mut impl Write,
) -> Result<()> {
    let msgs: Vec<_> = input
        .iter()
        .cloned()
        .map(BlindedMessageBorsh::from)
        .collect();
    borsh::BorshSerialize::serialize(&msgs, writer)?;
    Ok(())
}
pub fn deserialize_vecof_blindedmessage(
    reader: &mut impl Read,
) -> Result<Vec<cashu::BlindedMessage>> {
    let msgs: Vec<BlindedMessageBorsh> = borsh::BorshDeserialize::deserialize_reader(reader)?;
    Ok(msgs.into_iter().map(cashu::BlindedMessage::from).collect())
}

#[derive(Debug, Clone, borsh::BorshSerialize, borsh::BorshDeserialize)]
struct BlindSigDleq {
    e: String,
    s: String,
}
impl std::convert::From<cashu::BlindSignatureDleq> for BlindSigDleq {
    fn from(d: cashu::BlindSignatureDleq) -> Self {
        BlindSigDleq {
            e: d.e.to_string(),
            s: d.s.to_string(),
        }
    }
}
impl std::convert::TryFrom<BlindSigDleq> for cashu::BlindSignatureDleq {
    type Error = BorshError;
    fn try_from(d: BlindSigDleq) -> Result<Self> {
        let e = cashu::SecretKey::from_str(&d.e)
            .map_err(|e| BorshError::new(ErrorKind::InvalidData, e))?;
        let s = cashu::SecretKey::from_str(&d.s)
            .map_err(|e| BorshError::new(ErrorKind::InvalidData, e))?;
        Ok(cashu::BlindSignatureDleq { e, s })
    }
}

#[derive(Debug, Clone, borsh::BorshSerialize, borsh::BorshDeserialize)]
struct BlindSignatureBorsh {
    amount: u64,
    kid: Vec<u8>,
    c: [u8; secp256k1::constants::PUBLIC_KEY_SIZE],
    dleq: Option<BlindSigDleq>,
}
impl std::convert::From<cashu::BlindSignature> for BlindSignatureBorsh {
    fn from(sig: cashu::BlindSignature) -> Self {
        BlindSignatureBorsh {
            amount: u64::from(sig.amount),
            kid: sig.keyset_id.to_bytes(),
            c: sig.c.to_bytes(),
            dleq: sig.dleq.map(BlindSigDleq::from),
        }
    }
}
impl std::convert::TryFrom<BlindSignatureBorsh> for cashu::BlindSignature {
    type Error = BorshError;
    fn try_from(sig: BlindSignatureBorsh) -> Result<Self> {
        Ok(cashu::BlindSignature {
            amount: cashu::Amount::from(sig.amount),
            keyset_id: cashu::Id::from_bytes(&sig.kid).expect("keyset_id parse"),
            c: cashu::PublicKey::from_slice(&sig.c).expect("c parse"),
            dleq: sig
                .dleq
                .map(cashu::BlindSignatureDleq::try_from)
                .transpose()?,
        })
    }
}
pub fn serialize_vecof_blindsignature(
    input: &[cashu::BlindSignature],
    writer: &mut impl Write,
) -> Result<()> {
    let sigs: Vec<_> = input
        .iter()
        .cloned()
        .map(BlindSignatureBorsh::from)
        .collect();
    borsh::BorshSerialize::serialize(&sigs, writer)?;
    Ok(())
}
pub fn deserialize_vecof_blindsignature(
    reader: &mut impl Read,
) -> Result<Vec<cashu::BlindSignature>> {
    let sigs: Vec<BlindSignatureBorsh> = borsh::BorshDeserialize::deserialize_reader(reader)?;
    sigs.into_iter()
        .map(cashu::BlindSignature::try_from)
        .collect()
}
pub fn serialize_option_vecof_blindsignature(
    opt: &Option<Vec<cashu::BlindSignature>>,
    writer: &mut impl Write,
) -> Result<()> {
    match opt {
        None => borsh::BorshSerialize::serialize(&false, writer),
        Some(vec) => {
            borsh::BorshSerialize::serialize(&true, writer)?;
            serialize_vecof_blindsignature(vec, writer)
        }
    }
}
pub fn deserialize_option_vecof_blindsignature(
    reader: &mut impl Read,
) -> Result<Option<Vec<cashu::BlindSignature>>> {
    let is_some: bool = borsh::BorshDeserialize::deserialize_reader(reader)?;
    if is_some {
        let vec = deserialize_vecof_blindsignature(reader)?;
        Ok(Some(vec))
    } else {
        Ok(None)
    }
}

pub fn serialize_btc_amount(amount: &bitcoin::Amount, writer: &mut impl Write) -> Result<()> {
    serialize_as_u64(&amount.to_sat(), writer)
}

pub fn deserialize_btc_amount(reader: &mut impl Read) -> Result<bitcoin::Amount> {
    let sats = deserialize_from_u64(reader)?;
    Ok(bitcoin::Amount::from_sat(sats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_tests;

    #[test]
    fn serialize_deserialize_chrono_naivedate() {
        let date = chrono::NaiveDate::from_ymd_opt(2023, 10, 5).unwrap();
        let mut buf = Vec::new();
        serialize_as_str(&date, &mut buf).unwrap();
        let deserialized_date = deserialize_from_str(&mut buf.as_slice()).unwrap();
        assert_eq!(date, deserialized_date);
    }

    #[test]
    fn serialize_deserialize_btc_pubkey() {
        let pubkey_str = "02c0ded8f7b5e6c3f4e8b6a1e4f3c2d1e0f9e8d7c6b5a4e3f2d1c0b9a8e7f6d5c4";
        let pubkey = bitcoin::PublicKey::from_str(pubkey_str).unwrap();
        let mut buf = Vec::new();
        serialize_as_str(&pubkey, &mut buf).unwrap();
        let deserialized_pubkey = deserialize_from_str(&mut buf.as_slice()).unwrap();
        assert_eq!(pubkey, deserialized_pubkey);
    }

    #[test]
    fn serialize_deserialize_chrono_tstamp() {
        let tstamp = chrono::Utc::now();
        let mut buf = Vec::new();
        serialize_as_str(&tstamp, &mut buf).unwrap();
        let deserialized_tstamp: chrono::DateTime<chrono::Utc> =
            deserialize_from_str(&mut buf.as_slice()).unwrap();
        assert_eq!(tstamp, deserialized_tstamp);
    }

    #[test]
    fn serialize_deserialize_vec_of_jsons_cdk_proofs() {
        let (_, keyset) = core_tests::generate_random_ecash_keyset();
        let amount = cashu::Amount::from_str("1000").unwrap();
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amount.split());
        let mut buf = Vec::new();
        serialize_vec_of_jsons(&proofs, &mut buf).unwrap();
        let deserialized = deserialize_vec_of_jsons(&mut buf.as_slice()).unwrap();
        assert_eq!(proofs, deserialized);
    }

    #[test]
    fn serialize_deserialize_vec_of_strs_cdk_pubkeys() {
        let pks: Vec<_> = std::iter::repeat_with(|| {
            cashu::PublicKey::from(core_tests::generate_random_keypair().public_key())
        })
        .take(5)
        .collect();
        let mut buf = Vec::new();
        serialize_vec_of_strs(&pks, &mut buf).unwrap();
        let deserialized = deserialize_vec_of_strs(&mut buf.as_slice()).unwrap();
        assert_eq!(pks, deserialized);
    }

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Field {
        pub f1: String,
        pub f2: u8,
    }
    #[derive(borsh::BorshSerialize, borsh::BorshDeserialize, PartialEq, Debug)]
    struct Test {
        pub f1: String,
        #[borsh(
            serialize_with = "serialize_vec_of_jsons",
            deserialize_with = "deserialize_vec_of_jsons"
        )]
        pub f2: Vec<Field>,
        pub f3: Vec<u32>,
    }

    #[test]
    fn serialize_deserialize_struct() {
        let t = Test {
            f1: String::from("field 1"),
            f2: vec![
                Field {
                    f1: String::from("a"),
                    f2: 1,
                },
                Field {
                    f1: String::from("b"),
                    f2: 2,
                },
            ],
            f3: vec![10, 20, 30],
        };
        let mut buf = Vec::new();
        borsh::BorshSerialize::serialize(&t, &mut buf).unwrap();
        let deserialized_t =
            borsh::BorshDeserialize::deserialize_reader(&mut buf.as_slice()).unwrap();
        assert_eq!(t, deserialized_t);
    }

    #[test]
    fn serialize_deserialize_btc_amount() {
        let amount = bitcoin::Amount::from_sat(123456789);
        let mut buf = Vec::new();
        serialize_btc_amount(&amount, &mut buf).unwrap();
        let deserialized_amount = deserialize_btc_amount(&mut buf.as_slice()).unwrap();
        assert_eq!(amount, deserialized_amount);
    }

    #[test]
    fn serialize_deserialize_blinded_messages_and_signatures() {
        let (_, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts = cashu::Amount::from_str("1000").unwrap().split();

        // BlindedMessages
        let msgs: Vec<cashu::BlindedMessage> = amounts
            .iter()
            .map(|a| {
                let secret = cashu::secret::Secret::new(rand::random::<u64>().to_string());
                let (b_, _) = cashu::dhke::blind_message(secret.as_bytes(), None).unwrap();
                cashu::BlindedMessage {
                    amount: *a,
                    keyset_id: keyset.id,
                    blinded_secret: b_,
                    witness: None,
                }
            })
            .collect();
        let mut buf = Vec::new();
        serialize_vecof_blindedmessage(&msgs, &mut buf).unwrap();
        let deser_msgs = deserialize_vecof_blindedmessage(&mut buf.as_slice()).unwrap();
        assert_eq!(msgs, deser_msgs);
        // determinism: re-serializing produces identical bytes
        let mut buf2 = Vec::new();
        serialize_vecof_blindedmessage(&msgs, &mut buf2).unwrap();
        assert_eq!(buf, buf2);

        // BlindSignatures
        let sigs = core_tests::generate_ecash_signatures(&keyset, &amounts);
        buf.clear();
        serialize_vecof_blindsignature(&sigs, &mut buf).unwrap();
        let deser_sigs = deserialize_vecof_blindsignature(&mut buf.as_slice()).unwrap();
        assert_eq!(sigs, deser_sigs);

        // Option<Vec<BlindSignature>>
        buf.clear();
        serialize_option_vecof_blindsignature(&Some(sigs.clone()), &mut buf).unwrap();
        let deser_opt = deserialize_option_vecof_blindsignature(&mut buf.as_slice()).unwrap();
        assert_eq!(deser_opt, Some(sigs));

        buf.clear();
        serialize_option_vecof_blindsignature(&None, &mut buf).unwrap();
        let deser_none = deserialize_option_vecof_blindsignature(&mut buf.as_slice()).unwrap();
        assert_eq!(deser_none, None);
    }
}
