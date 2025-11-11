// ----- standard library imports
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
    #[error("no key for amount {0}")]
    NoKeyForAmount(cashu::Amount),
    #[error("cdk::dhke {0}")]
    CdkDHKE(#[from] cashu::dhke::Error),
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
    let Some(key) = keys.keys.amount_key(signature.amount) else {
        return Err(ECashSignatureError::NoKeyForAmount(signature.amount));
    };
    let c = cashu::dhke::unblind_message(&signature.c, &premint.r, &key)?;
    let mut proof = cashu::Proof::new(signature.amount, keys.id, premint.secret.clone(), c);
    if let Some(dleq) = signature.dleq {
        proof.dleq = Some(cashu::ProofDleq::new(dleq.e, dleq.s, premint.r.clone()));
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
                proof.verify_htlc()?;
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
    let y = fp.c.mul_tweak(secp::global::SECP256K1, &scalar)?;
    if y == fp.y {
        Ok(())
    } else {
        Err(ECashSignatureError::Invalid)
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
}
