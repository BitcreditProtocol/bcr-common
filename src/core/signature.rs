// ----- standard library imports
// ----- extra library imports
use bitcoin::{
    hashes::{Hash, sha256::Hash as Sha256},
    secp256k1 as secp,
};
use borsh::BorshSerialize;
use thiserror::Error;
// ----- local modules

// ----- end imports

pub type SignatureResult<T> = std::result::Result<T, Error>;
#[derive(Debug, Error)]
pub enum Error {
    #[error("Borsh error {0}")]
    Borsh(#[from] borsh::io::Error),
    #[error("Secp256k1 error {0}")]
    Secp256k1(#[from] secp::Error),
}

pub fn sign_with_key(
    msg: &impl BorshSerialize,
    keys: &secp::Keypair,
) -> SignatureResult<secp::schnorr::Signature> {
    let serialized = borsh::to_vec(&msg)?;
    let sha = Sha256::hash(&serialized);
    let secp_msg = secp::Message::from_digest(*sha.as_ref());
    Ok(secp::global::SECP256K1.sign_schnorr(&secp_msg, keys))
}

pub fn verify_with_key(
    msg: &impl BorshSerialize,
    signature: &secp::schnorr::Signature,
    key: &secp::XOnlyPublicKey,
) -> SignatureResult<()> {
    let serialized = borsh::to_vec(&msg)?;
    let sha = Sha256::hash(&serialized);
    let secp_msg = secp::Message::from_digest(*sha.as_ref());
    secp::global::SECP256K1.verify_schnorr(signature, &secp_msg, key)?;
    Ok(())
}
