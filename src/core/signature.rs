// ----- standard library imports
use std::{collections::HashMap, convert::Infallible, str::FromStr};
// ----- extra library imports
use bitcoin::{
    base64::prelude::*,
    hashes::{Hash, sha256::Hash as Sha256},
    secp256k1 as secp,
};
use borsh::{BorshDeserialize, BorshSerialize};
use cashu::{nut10 as cdk10, nut11 as cdk11, nut12 as cdk12, nut14 as cdk14};
use thiserror::Error;
// ----- local modules

// ----- end imports

pub type BorshMsgSignatureResult<T> = std::result::Result<T, BorshMsgSignatureError>;
#[derive(Debug, Error)]
pub enum BorshMsgSignatureError {
    #[error("Borsh {0}")]
    Borsh(#[from] borsh::io::Error),
    #[error("Secp256k1 {0}")]
    Secp256k1(#[from] secp::Error),
    #[error("base64 {0}")]
    Base64Decode(#[from] bitcoin::base64::DecodeError),
}

// Sign a Borsh-serializable message with a secp256k1 keypair using Schnorr signatures.
// Returns the Base64 serialized message and the signature.
pub fn serialize_n_schnorr_sign_borsh_msg(
    msg: &impl BorshSerialize,
    keys: &secp::Keypair,
) -> BorshMsgSignatureResult<(String, secp::schnorr::Signature)> {
    let serialized = borsh::to_vec(msg)?;
    let sha = Sha256::hash(&serialized);
    let secp_msg = secp::Message::from_digest(*sha.as_ref());
    let signature = secp::global::SECP256K1.sign_schnorr(&secp_msg, keys);
    let b64 = BASE64_STANDARD.encode(serialized);
    Ok((b64, signature))
}

pub fn serialize_borsh_msg_b64(msg: &impl BorshSerialize) -> BorshMsgSignatureResult<String> {
    Ok(BASE64_STANDARD.encode(borsh::to_vec(msg)?))
}

// deserialization and signature verification is split into two parts
// sometimes the public key is embedded in the message itself
pub fn deserialize_borsh_msg<Message: BorshDeserialize>(
    payload: &str,
) -> BorshMsgSignatureResult<Message> {
    let serialized = BASE64_STANDARD.decode(payload)?;
    let message: Message = borsh::from_slice(&serialized)?;
    Ok(message)
}

pub fn schnorr_verify_b64(
    payload: &str,
    signature: &secp::schnorr::Signature,
    key: &secp::XOnlyPublicKey,
) -> BorshMsgSignatureResult<()> {
    let serialized = BASE64_STANDARD.decode(payload)?;
    let sha = Sha256::hash(&serialized);
    let secp_msg = secp::Message::from_digest(*sha.as_ref());
    secp::global::SECP256K1.verify_schnorr(signature, &secp_msg, key)?;
    Ok(())
}

pub type ECashSignatureResult<T> = std::result::Result<T, ECashSignatureError>;
#[derive(Debug, Error)]
pub enum ECashSignatureError {
    #[error("Invalid signature")]
    Invalid,
    #[error("mismatched keyset {0} {1}")]
    MismatchedKid(cashu::Id, cashu::Id),
    #[error("mismatched amount {0} {1}")]
    MismatchedAmount(cashu::Amount, cashu::Amount),
    #[error("no key for amount {0}")]
    NoKeyForAmount(cashu::Amount),
    #[error("cdk::dhke {0}")]
    CdkDHKE(#[from] cashu::dhke::Error),
    #[error("Nut10 {0}")]
    Cdk10(#[from] cdk10::Error),
    #[error("Nut11 {0}")]
    Cdk11(#[from] cdk11::Error),
    #[error("cdk::nut12 {0}")]
    Cdk12(#[from] cdk12::Error),
    #[error("Nut14 {0}")]
    Cdk14(#[from] cdk14::Error),
    #[error("secp256k1 {0}")]
    Secp256k1(#[from] secp::Error),
}

pub fn sign_ecash(
    keyset: &cashu::MintKeySet,
    blind: &cashu::BlindedMessage,
) -> ECashSignatureResult<cashu::BlindSignature> {
    let key = keyset
        .keys
        .get(&blind.amount)
        .ok_or(ECashSignatureError::NoKeyForAmount(blind.amount))?;
    let raw_signature = cashu::dhke::sign_message(&key.secret_key, &blind.blinded_secret)?;
    let mut signature = cashu::BlindSignature {
        amount: blind.amount,
        c: raw_signature,
        keyset_id: keyset.id,
        dleq: None,
    };
    signature.add_dleq_proof(&blind.blinded_secret, &key.secret_key)?;
    Ok(signature)
}

pub fn unblind_ecash_signature(
    keys: &cashu::KeySet,
    premint: cashu::PreMint,
    signature: cashu::BlindSignature,
) -> ECashSignatureResult<cashu::Proof> {
    if signature.keyset_id != keys.id {
        return Err(ECashSignatureError::MismatchedKid(
            signature.keyset_id,
            keys.id,
        ));
    }
    if premint.blinded_message.keyset_id != keys.id {
        return Err(ECashSignatureError::MismatchedKid(
            premint.blinded_message.keyset_id,
            keys.id,
        ));
    }
    if premint.amount != cashu::Amount::ZERO && premint.amount != signature.amount {
        return Err(ECashSignatureError::MismatchedAmount(
            premint.amount,
            signature.amount,
        ));
    }
    let Some(key) = keys.keys.amount_key(signature.amount) else {
        return Err(ECashSignatureError::NoKeyForAmount(signature.amount));
    };
    let c = cashu::dhke::unblind_message(&signature.c, &premint.r, &key)?;
    let mut proof = cashu::Proof::new(signature.amount, keys.id, premint.secret, c);
    if let Some(dleq) = signature.dleq {
        proof.dleq = Some(cashu::ProofDleq::new(dleq.e, dleq.s, premint.r));
    }
    Ok(proof)
}

pub fn verify_ecash_proof(
    keyset: &cashu::MintKeySet,
    proof: &cashu::Proof,
) -> ECashSignatureResult<()> {
    if proof.keyset_id != keyset.id {
        return Err(ECashSignatureError::MismatchedKid(
            keyset.id,
            proof.keyset_id,
        ));
    }
    // ref: https://docs.rs/cdk/latest/cdk/mint/struct.Mint.html#method.verify_proofs
    if let Ok(secret) = <&cashu::secret::Secret as TryInto<cdk10::Secret>>::try_into(&proof.secret)
    {
        match secret.kind() {
            cashu::nuts::Kind::P2PK => {
                proof.verify_p2pk()?;
            }
            cashu::nuts::Kind::HTLC => {
                verify_exchange_htlc(proof)?;
            }
        }
    }
    let keypair = keyset
        .keys
        .get(&proof.amount)
        .ok_or(ECashSignatureError::NoKeyForAmount(proof.amount))?;
    cashu::dhke::verify_message(&keypair.secret_key, proof.c, proof.secret.as_bytes())?;
    Ok(())
}

// Tag stamped into an offline-exchange HTLC secret at issuance so verification routes to the
// raw-bytes `verify_offline_exchange_htlc` rather than cashu's `verify_htlc`.
pub const EXCHANGE_TAG: &str = "exchange";
pub const OFFLINE: &str = "offline";

// Build a cashu secret for an offline-exchange HTLC, tagged `[EXCHANGE_TAG, OFFLINE]`. cashu
// ignores the custom tag when parsing conditions, so both verifiers still read pubkeys/locktime.
pub fn offline_htlc_secret(
    conditions: cashu::SpendingConditions,
) -> ECashSignatureResult<cashu::secret::Secret> {
    let nut10: cdk10::Secret = conditions.into();
    let mut tags = nut10.secret_data().tags().cloned().unwrap_or_default();
    tags.push(vec![EXCHANGE_TAG.to_string(), OFFLINE.to_string()]);
    let tagged = cdk10::Secret::new(
        nut10.kind(),
        cdk10::SecretData::new(nut10.secret_data().data(), Some(tags)),
    );
    Ok(tagged.try_into()?)
}

// True if the proof's HTLC secret carries the offline-exchange tag.
pub fn is_offline_exchange_htlc(proof: &cashu::Proof) -> bool {
    let Ok(secret) = <&cashu::secret::Secret as TryInto<cdk10::Secret>>::try_into(&proof.secret)
    else {
        return false;
    };
    secret.secret_data().tags().is_some_and(|tags| {
        tags.iter().any(|t| {
            t.first().map(String::as_str) == Some(EXCHANGE_TAG)
                && t.get(1).map(String::as_str) == Some(OFFLINE)
        })
    })
}

// Route HTLC verification by the committed offline-exchange tag: tagged proofs use the
// raw-bytes verifier, everything else uses cashu's `verify_htlc`.
pub fn verify_exchange_htlc(proof: &cashu::Proof) -> ECashSignatureResult<()> {
    if is_offline_exchange_htlc(proof) {
        verify_offline_exchange_htlc(proof)
    } else {
        proof.verify_htlc()?;
        Ok(())
    }
}

// Verify an HTLC proof from the offline intermint exchange, where the preimage is the original
// alpha proof's secret string rather than a fixed-size 32-byte hex value. Reached via
// `verify_exchange_htlc` for tagged proofs; online and generic HTLCs use cashu's `verify_htlc`.
pub fn verify_offline_exchange_htlc(proof: &cashu::Proof) -> ECashSignatureResult<()> {
    // Only HTLC spending conditions are valid here; anything else short-circuits.
    let spending: cashu::SpendingConditions = (&proof.secret)
        .try_into()
        .map_err(|_| cdk14::Error::IncorrectSecretKind)?;
    let cashu::SpendingConditions::HTLCConditions {
        data: hash_lock,
        conditions,
    } = spending
    else {
        return Err(cdk14::Error::IncorrectSecretKind.into());
    };

    let Some(cashu::Witness::HTLCWitness(htlc_witness)) = &proof.witness else {
        return Err(cdk14::Error::IncorrectSecretKind.into());
    };

    if let Some(conditions) = conditions {
        // Refund keys are only valid once the locktime has passed.
        if let Some(locktime) = conditions.locktime
            && locktime < cashu::util::unix_time()
        {
            if conditions.refund_keys.is_none() {
                return Ok(());
            }
            if let Some(refund_keys) = conditions.refund_keys {
                let signatures = parse_signatures(htlc_witness.signatures.as_deref())?;
                if valid_signatures(proof.secret.as_bytes(), &refund_keys, &signatures) >= 1 {
                    return Ok(());
                }
            }
        }
        if let Some(pubkeys) = conditions.pubkeys {
            let required = conditions.num_sigs.unwrap_or(1);
            let signatures = parse_signatures(htlc_witness.signatures.as_deref())?;
            if valid_signatures(proof.secret.as_bytes(), &pubkeys, &signatures) < required {
                return Err(cdk14::Error::SpendConditionsNotMet.into());
            }
        }
    }

    let preimage_hash = Sha256::hash(htlc_witness.preimage.as_bytes());
    if hash_lock != preimage_hash {
        return Err(cdk14::Error::Preimage.into());
    }
    Ok(())
}

fn parse_signatures(
    signatures: Option<&[String]>,
) -> ECashSignatureResult<Vec<secp::schnorr::Signature>> {
    signatures
        .ok_or(cdk14::Error::SignaturesNotProvided)?
        .iter()
        .map(|s| secp::schnorr::Signature::from_str(s))
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn valid_signatures(
    msg: &[u8],
    pubkeys: &[cashu::PublicKey],
    signatures: &[secp::schnorr::Signature],
) -> u64 {
    pubkeys
        .iter()
        .filter(|pubkey| signatures.iter().any(|sig| pubkey.verify(msg, sig).is_ok()))
        .count() as u64
}

pub struct ProofFingerprint {
    pub keyset_id: cashu::Id,
    pub amount: cashu::Amount,
    pub c: secp::PublicKey,
    pub y: secp::PublicKey,
}

pub fn verify_ecash_fingerprint(
    keyset: &cashu::MintKeySet,
    fp: &ProofFingerprint,
) -> ECashSignatureResult<()> {
    if fp.keyset_id != keyset.id {
        return Err(ECashSignatureError::MismatchedKid(keyset.id, fp.keyset_id));
    }
    let Some(key) = keyset.keys.get(&fp.amount) else {
        return Err(ECashSignatureError::NoKeyForAmount(fp.amount));
    };
    let scalar = key.secret_key.clone().to_scalar();
    let expected_c = fp.y.mul_tweak(secp::global::SECP256K1, &scalar)?;
    if expected_c == fp.c {
        Ok(())
    } else {
        Err(ECashSignatureError::Invalid)
    }
}

pub fn proofs_to_map(
    proofs: impl IntoIterator<Item = cashu::Proof>,
) -> HashMap<cashu::Id, Vec<cashu::Proof>> {
    let mut map: HashMap<cashu::Id, Vec<cashu::Proof>> = HashMap::new();
    for proof in proofs.into_iter() {
        map.entry(proof.keyset_id).or_default().push(proof);
    }
    map
}

pub trait ToFingerPrint {
    type Error;
    fn to_fp(&self) -> std::result::Result<secp::PublicKey, Self::Error>;
}
impl ToFingerPrint for cashu::Proof {
    type Error = cashu::nut00::Error;
    fn to_fp(&self) -> std::result::Result<secp::PublicKey, Self::Error> {
        let y = self.y()?;
        Ok(*y)
    }
}
impl ToFingerPrint for ProofFingerprint {
    type Error = Infallible;
    fn to_fp(&self) -> std::result::Result<secp::PublicKey, Self::Error> {
        Ok(self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_borsh_schnorr_sign_verify() {
        #[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug)]
        struct TestMessage {
            id: u32,
            content: String,
        }
        let keypair = secp::Keypair::new_global(&mut rand::thread_rng());
        let xonly_pk = secp::XOnlyPublicKey::from_keypair(&keypair).0;
        let msg = TestMessage {
            id: 42,
            content: "Hello, world!".to_string(),
        };
        let (b64_msg, signature) =
            serialize_n_schnorr_sign_borsh_msg(&msg, &keypair).expect("Signing failed");
        let deserialized_msg: TestMessage =
            deserialize_borsh_msg(&b64_msg).expect("Deserialization failed");
        assert_eq!(msg, deserialized_msg);
        schnorr_verify_b64(&b64_msg, &signature, &xonly_pk).expect("Verification failed");
    }

    #[test]
    fn test_verify_ecash_fingerprint() {
        use crate::core::test_utils::{generate_random_ecash_keyset, generate_random_ecash_proofs};

        let (_, keyset) = generate_random_ecash_keyset();
        let proofs = generate_random_ecash_proofs(&keyset, &[cashu::Amount::from(1u64)]);
        let proof = &proofs[0];
        let y = proof.y().expect("hash_to_curve");

        let valid_fp = ProofFingerprint {
            keyset_id: proof.keyset_id,
            amount: proof.amount,
            c: *proof.c,
            y: *y,
        };
        verify_ecash_fingerprint(&keyset, &valid_fp).expect("valid fingerprint");

        let invalid_fp = ProofFingerprint {
            c: secp::Keypair::new_global(&mut rand::thread_rng()).public_key(),
            ..valid_fp
        };
        assert!(verify_ecash_fingerprint(&keyset, &invalid_fp).is_err());
    }

    fn offline_htlc_proof(wallet: &cashu::SecretKey, preimage: &str) -> cashu::Proof {
        use crate::core::test_utils::generate_random_ecash_keyset;

        let (_, keyset) = generate_random_ecash_keyset();
        let hash_lock = Sha256::hash(preimage.as_bytes());
        let locktime = cashu::util::unix_time() + 3600;
        let conditions = cashu::Conditions::new(
            Some(locktime),
            Some(vec![wallet.public_key()]),
            None,
            Some(1),
            None,
            None,
        )
        .expect("conditions");
        let spending =
            cashu::SpendingConditions::new_htlc_hash(&hash_lock.to_string(), Some(conditions))
                .expect("htlc conditions");
        let secret: cashu::secret::Secret = spending.try_into().expect("secret");

        let mut proof = cashu::Proof::new(
            cashu::Amount::from(1u64),
            keyset.id,
            secret,
            wallet.public_key(),
        );
        let signature = wallet.sign(&proof.secret.to_bytes()).expect("sign");
        proof.witness = Some(cashu::Witness::HTLCWitness(cashu::HTLCWitness {
            preimage: preimage.to_string(),
            signatures: Some(vec![signature.to_string()]),
        }));
        proof
    }

    #[test]
    fn test_verify_offline_exchange_htlc() {
        let kp = secp::Keypair::new_global(&mut rand::thread_rng());
        let wallet: cashu::SecretKey = kp.secret_key().into();
        let preimage = "this is not a fixed size hex preimage";

        let proof = offline_htlc_proof(&wallet, preimage);
        verify_offline_exchange_htlc(&proof).expect("valid offline htlc");

        let mut wrong_preimage = proof.clone();
        wrong_preimage.witness = Some(cashu::Witness::HTLCWitness(cashu::HTLCWitness {
            preimage: "a different preimage".to_string(),
            signatures: proof.witness.as_ref().and_then(|w| w.signatures()),
        }));
        assert!(verify_offline_exchange_htlc(&wrong_preimage).is_err());

        let other = secp::Keypair::new_global(&mut rand::thread_rng());
        let other_key: cashu::SecretKey = other.secret_key().into();
        let bad_sig = other_key.sign(&proof.secret.to_bytes()).expect("sign");
        let mut wrong_sig = proof.clone();
        wrong_sig.witness = Some(cashu::Witness::HTLCWitness(cashu::HTLCWitness {
            preimage: preimage.to_string(),
            signatures: Some(vec![bad_sig.to_string()]),
        }));
        assert!(verify_offline_exchange_htlc(&wrong_sig).is_err());
    }

    #[test]
    fn test_verify_exchange_htlc_routing() {
        use crate::core::test_utils::generate_random_ecash_keyset;

        let (_, keyset) = generate_random_ecash_keyset();
        let kp = secp::Keypair::new_global(&mut rand::thread_rng());
        let wallet: cashu::SecretKey = kp.secret_key().into();

        let conditions = || {
            cashu::Conditions::new(
                Some(cashu::util::unix_time() + 3600),
                Some(vec![wallet.public_key()]),
                None,
                Some(1),
                None,
                None,
            )
            .expect("conditions")
        };
        let mk_htlc = |secret: cashu::secret::Secret, preimage: &str| -> cashu::Proof {
            let mut p = cashu::Proof::new(
                cashu::Amount::from(1u64),
                keyset.id,
                secret,
                wallet.public_key(),
            );
            let sig = wallet.sign(&p.secret.to_bytes()).expect("sign");
            p.witness = Some(cashu::Witness::HTLCWitness(cashu::HTLCWitness {
                preimage: preimage.to_string(),
                signatures: Some(vec![sig.to_string()]),
            }));
            p
        };

        // Offline: tagged, plain 64-hex preimage with a raw-bytes lock (the case the
        // shape heuristic mis-routed). Must route to the raw-bytes verifier and pass.
        let preimage = "aa".repeat(32);
        let hash_lock = Sha256::hash(preimage.as_bytes());
        let spending =
            cashu::SpendingConditions::new_htlc_hash(&hash_lock.to_string(), Some(conditions()))
                .expect("htlc");
        let offline = mk_htlc(
            offline_htlc_secret(spending).expect("tagged secret"),
            &preimage,
        );
        assert!(is_offline_exchange_htlc(&offline));
        verify_exchange_htlc(&offline).expect("offline routes to raw-bytes verifier");

        // Generic/online: untagged, 32-byte-hex preimage with a cashu lock.
        let hex_preimage = "07".repeat(32);
        let spending =
            cashu::SpendingConditions::new_htlc(hex_preimage.clone(), Some(conditions()))
                .expect("htlc");
        let secret: cashu::secret::Secret = spending.try_into().expect("secret");
        let online = mk_htlc(secret, &hex_preimage);
        assert!(!is_offline_exchange_htlc(&online));
        verify_exchange_htlc(&online).expect("untagged routes to cashu");
    }
}
