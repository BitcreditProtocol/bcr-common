// ----- standard library imports
// ----- extra library imports
use bitcoin::secp256k1 as secp;
use thiserror::Error;
// ----- local modules
mod billid;
pub mod keys;
mod nodeid;
pub mod signature;
#[cfg(any(feature = "wallet", feature = "mint"))]
pub mod swap;
#[cfg(feature = "test-utils")]
pub mod test_utils;

// ----- end imports

#[derive(Debug, Error)]
pub enum Error {
    /// errors stemming from providing an invalid node id
    #[error("Invalid NodeId")]
    InvalidNodeId,
    /// errors stemming from providing an invalid bill id
    #[error("Invalid BillId")]
    InvalidBillId,
}

pub use billid::BillId;
pub use nodeid::NodeId;

pub const ID_PREFIX: &str = "bitcr";
pub const NETWORK_MAINNET: char = 'm';
pub const NETWORK_TESTNET: char = 't';
pub const NETWORK_TESTNET4: char = 'T';
pub const NETWORK_REGTEST: char = 'r';
pub const NETWORK_SIGNET: char = 's';

pub fn network_char(network: &bitcoin::Network) -> char {
    match network {
        bitcoin::Network::Bitcoin => NETWORK_MAINNET,
        bitcoin::Network::Testnet => NETWORK_TESTNET,
        bitcoin::Network::Testnet4 => NETWORK_TESTNET4,
        bitcoin::Network::Signet => NETWORK_SIGNET,
        bitcoin::Network::Regtest => NETWORK_REGTEST,
    }
}

pub fn network_from_char(c: char) -> Option<bitcoin::Network> {
    match c {
        NETWORK_MAINNET => Some(bitcoin::Network::Bitcoin),
        NETWORK_TESTNET => Some(bitcoin::Network::Testnet),
        NETWORK_TESTNET4 => Some(bitcoin::Network::Testnet4),
        NETWORK_SIGNET => Some(bitcoin::Network::Signet),
        NETWORK_REGTEST => Some(bitcoin::Network::Regtest),
        _ => None,
    }
}

pub fn generate_random_keypair() -> secp::Keypair {
    let mut rng = rand::thread_rng();
    secp::Keypair::new(secp::global::SECP256K1, &mut rng)
}
