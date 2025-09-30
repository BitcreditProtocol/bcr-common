// ----- standard library imports
// ----- extra library imports
use thiserror::Error;
// ----- local modules
mod billid;
mod nodeid;
pub mod signature;
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

fn network_char(network: &bitcoin::Network) -> char {
    match network {
        bitcoin::Network::Bitcoin => NETWORK_MAINNET,
        bitcoin::Network::Testnet => NETWORK_TESTNET,
        bitcoin::Network::Testnet4 => NETWORK_TESTNET4,
        bitcoin::Network::Signet => unreachable!(),
        bitcoin::Network::Regtest => NETWORK_REGTEST,
    }
}
