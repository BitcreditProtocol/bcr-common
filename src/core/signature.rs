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
pub fn ser_n_sign_borsh_msg(
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

pub fn deser_n_verify_borsh_msg<Message: BorshDeserialize>(
    payload: &str,
    signature: &secp::schnorr::Signature,
    key: &secp::XOnlyPublicKey,
) -> BorshMsgSignatureResult<Message> {
    let serialized = BASE64_STANDARD.decode(payload)?;
    let sha = Sha256::hash(&serialized);
    let secp_msg = secp::Message::from_digest(*sha.as_ref());
    secp::global::SECP256K1.verify_schnorr(signature, &secp_msg, key)?;
    let message: Message = borsh::from_slice(&serialized)?;
    Ok(message)
}

pub type ECashSignatureResult<T> = std::result::Result<T, ECashSignatureError>;
#[derive(Debug, Error)]
pub enum ECashSignatureError {
    #[error("no key for amount {0}")]
    NoKeyForAmount(cashu::Amount),
    #[error("cdk::dhke error {0}")]
    CdkDHKE(#[from] cashu::dhke::Error),
    #[error("Nut11 error {0}")]
    Cdk11(#[from] cdk11::Error),
    #[error("cdk::nut12 error {0}")]
    Cdk12(#[from] cdk12::Error),
    #[error("Nut14 error {0}")]
    Cdk14(#[from] cdk14::Error),
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

pub fn verify_ecash(keyset: &cashu::MintKeySet, proof: &cashu::Proof) -> ECashSignatureResult<()> {
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
