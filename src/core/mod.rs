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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    const NETWORKS: [bitcoin::Network; 5] = [
        bitcoin::Network::Bitcoin,
        bitcoin::Network::Testnet,
        bitcoin::Network::Testnet4,
        bitcoin::Network::Signet,
        bitcoin::Network::Regtest,
    ];

    /// Every network must render to a char and parse back, Signet included: it
    /// used to make `network_char` panic
    #[test]
    fn network_char_round_trips_every_network() {
        for network in NETWORKS {
            assert_eq!(network_from_char(network_char(&network)), Some(network));
        }
        assert_eq!(network_from_char('x'), None);
        assert_eq!(network_from_char('B'), None);
    }

    #[test]
    fn ids_round_trip_every_network() {
        let pub_key = generate_random_keypair().public_key();
        for network in NETWORKS {
            let node_id = NodeId::new(pub_key, network);
            assert_eq!(NodeId::from_str(&node_id.to_string()).unwrap(), node_id);
            assert_eq!(node_id.network(), network);

            let bill_id = BillId::new(pub_key, network);
            assert_eq!(BillId::from_str(&bill_id.to_string()).unwrap(), bill_id);
            assert_eq!(bill_id.network(), network);
        }
    }
}
