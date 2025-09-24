// ----- standard library imports
// ----- extra library imports
// ----- local modules
mod billid;
mod nodeid;
#[cfg(feature = "test-utils")]
pub mod test_utils;

// ----- end imports

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
