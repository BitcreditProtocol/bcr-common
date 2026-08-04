// ----- standard library imports
use std::str::FromStr;
// ----- extra library imports
use bitcoin::{
    base58,
    hashes::{Hash, sha256},
};
// ----- local modules
use crate::core::{Error, ID_PREFIX, network_char, network_from_char};

// ----- end imports

/// A bitcr Bill ID of the format <prefix><network><hash>
/// Example: bitcrtBBT5a1eNZ8zEUkU2rppXBDrZJjARoxPkZtBgFo2RLz3y
/// The prefix is bitcr
/// The pub key is a base58 encoded, sha256 hashed Secp256k1 public key (the bill pub key)
/// The network character can be parsed like this:
/// * m => Mainnet
/// * t => Testnet
/// * T => Testnet4
/// * r => Regtest
/// * s => Signet
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct BillId {
    hash: String,
    network: bitcoin::Network,
}

impl BillId {
    pub fn new(public_key: bitcoin::secp256k1::PublicKey, network: bitcoin::Network) -> Self {
        let raw_hash = sha256::Hash::hash(public_key.serialize().as_slice());
        let hash = base58::encode(raw_hash.as_byte_array());
        Self { hash, network }
    }

    pub fn network(&self) -> bitcoin::Network {
        self.network
    }
}

impl std::fmt::Display for BillId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            ID_PREFIX,
            network_char(&self.network),
            self.hash
        )
    }
}

impl FromStr for BillId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(ID_PREFIX) {
            return Err(Error::InvalidBillId);
        }
        let network = s
            .chars()
            .nth(ID_PREFIX.len())
            .and_then(network_from_char)
            .ok_or(Error::InvalidBillId)?;
        let hash_str = &s[ID_PREFIX.len() + 1..];
        let decoded = base58::decode(hash_str).map_err(|_| Error::InvalidBillId)?;
        if decoded.len() != sha256::Hash::LEN {
            return Err(Error::InvalidBillId);
        }
        Ok(Self {
            hash: hash_str.to_owned(),
            network,
        })
    }
}

impl serde::Serialize for BillId {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        s.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for BillId {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <std::string::String as serde::Deserialize>::deserialize(d)?;
        BillId::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl borsh::BorshSerialize for BillId {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let bill_id_str = self.to_string();
        borsh::BorshSerialize::serialize(&bill_id_str, writer)
    }
}

impl borsh::BorshDeserialize for BillId {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let bill_id_str: String = borsh::BorshDeserialize::deserialize_reader(reader)?;
        BillId::from_str(&bill_id_str)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
