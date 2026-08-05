// ----- standard library imports
// ----- extra library imports
use cashu::nut02::ShortKeysetId;
use thiserror::Error;
// ----- local modules
mod cbor;
mod proof;
mod token;
// ----- end imports

pub use proof::*;
pub use token::*;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("unsupported token")]
    UnsupportedToken,
    #[error("unknown token version {0:?}")]
    UnknownVersion(char),
    #[error("unknown network {0:?}")]
    UnknownNetwork(char),
    #[error("duplicate proof secret in token")]
    DuplicateProofs,
    #[error("keyset id {0} is not advertised by the mint")]
    UnknownKeysetId(ShortKeysetId),
    #[error("keyset id {0} matches more than one mint keyset")]
    AmbiguousKeysetId(ShortKeysetId),
    #[error("secp256k1 {0}")]
    Secp256k1(#[from] bitcoin::secp256k1::Error),
    #[error("invalid pubkey {0}")]
    InvalidPubkey(#[from] cashu::nut01::Error),
    #[error("invalid keyset id {0}")]
    InvalidKeysetId(#[from] cashu::nut02::Error),
    #[error("base64 {0}")]
    Base64(#[from] bitcoin::base64::DecodeError),
    #[error("cbor decode {0}")]
    CborDecode(#[from] ciborium::de::Error<std::io::Error>),
    #[error("cbor encode {0}")]
    CborEncode(#[from] ciborium::ser::Error<std::io::Error>),
    #[error("amount {0}")]
    Amount(#[from] cashu::amount::Error),
}
