// ----- standard library imports
// ----- extra library imports
use thiserror::Error;
// ----- local modules
pub mod taproot;
// ----- end imports

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid pubkey")]
    InvalidPubkey,
    #[error("secp256k1 {0}")]
    Secp256k1(#[from] bitcoin::secp256k1::Error),
    #[error("scalar out of range {0}")]
    ScalarOutOfRange(#[from] bitcoin::secp256k1::scalar::OutOfRangeError),
    #[error("taproot builder {0}")]
    TaprootBuilder(#[from] bitcoin::taproot::TaprootBuilderError),
    #[error("incomplete taproot tree")]
    IncompleteTaprootTree,
}
