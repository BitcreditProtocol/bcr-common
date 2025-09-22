// ----- standard library imports
use std::str::FromStr;
// ----- extra library imports
// ----- local modules
use crate::core::{
    ID_PREFIX, NETWORK_MAINNET, NETWORK_REGTEST, NETWORK_TESTNET, NETWORK_TESTNET4, network_char,
};

// ----- end imports

/// A bitcr Node ID of the format <prefix><network><pub_key>
/// Example: bitcrt039180c169e5f6d7c579cf1cefa37bffd47a2b389c8125601f4068c87bea795943
/// The prefix is bitcr
/// The pub key is a secp256k1 public key
/// The network character can be parsed like this:
/// * m => Mainnet
/// * t => Testnet
/// * T => Testnet4
/// * r => Regtest
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct NodeId {
    pub_key: bitcoin::secp256k1::PublicKey,
    network: bitcoin::Network,
}

impl NodeId {
    pub fn new(pub_key: bitcoin::secp256k1::PublicKey, network: bitcoin::Network) -> Self {
        Self { pub_key, network }
    }

    pub fn network(&self) -> bitcoin::Network {
        self.network
    }

    pub fn pub_key(&self) -> bitcoin::secp256k1::PublicKey {
        self.pub_key
    }

    pub fn npub(&self) -> nostr::PublicKey {
        nostr::PublicKey::from(self.pub_key.x_only_public_key().0)
    }

    pub fn equals_npub(&self, npub: &nostr::PublicKey) -> bool {
        self.npub() == *npub
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            ID_PREFIX,
            network_char(&self.network),
            self.pub_key
        )
    }
}

impl FromStr for NodeId {
    type Err = std::fmt::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(ID_PREFIX) {
            return Err(Self::Err {});
        }

        let network = match s.chars().nth(ID_PREFIX.len()) {
            None => {
                return Err(Self::Err {});
            }
            Some(network_str) => match network_str {
                NETWORK_MAINNET => bitcoin::Network::Bitcoin,
                NETWORK_TESTNET => bitcoin::Network::Testnet,
                NETWORK_TESTNET4 => bitcoin::Network::Testnet4,
                NETWORK_REGTEST => bitcoin::Network::Regtest,
                _ => {
                    return Err(Self::Err {});
                }
            },
        };

        let pub_key_str = &s[ID_PREFIX.len() + 1..];
        let pub_key =
            bitcoin::secp256k1::PublicKey::from_str(pub_key_str).map_err(|_| Self::Err {})?;

        Ok(Self { pub_key, network })
    }
}

impl serde::Serialize for NodeId {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        s.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for NodeId {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        NodeId::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl borsh::BorshSerialize for NodeId {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let node_id_str = self.to_string();
        borsh::BorshSerialize::serialize(&node_id_str, writer)
    }
}

impl borsh::BorshDeserialize for NodeId {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let node_id_str: String = borsh::BorshDeserialize::deserialize_reader(reader)?;
        NodeId::from_str(&node_id_str)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
