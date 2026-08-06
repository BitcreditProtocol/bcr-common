// ----- standard library imports
// ----- extra library imports
use serde::{Deserialize, Deserializer, Serializer};
// ----- local modules
use crate::wallet::Result;

// ----- end imports

/// A value that travels inside a token as a raw cbor byte string rather than as
/// its (much longer) textual form. Use with `#[serde(with = "crate::wallet::cbor")]`.
pub trait CborBytes: Sized {
    fn to_cbor_bytes(&self) -> Vec<u8>;
    fn from_cbor_bytes(bytes: &[u8]) -> Result<Self>;
}

pub fn serialize<T, S>(value: &T, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    T: CborBytes,
    S: Serializer,
{
    serializer.serialize_bytes(&value.to_cbor_bytes())
}

pub fn deserialize<'de, T, D>(deserializer: D) -> std::result::Result<T, D::Error>
where
    T: CborBytes,
    D: Deserializer<'de>,
{
    let bytes = Vec::<u8>::deserialize(deserializer)?;
    T::from_cbor_bytes(&bytes).map_err(serde::de::Error::custom)
}

impl CborBytes for bitcoin::secp256k1::PublicKey {
    fn to_cbor_bytes(&self) -> Vec<u8> {
        self.serialize().to_vec()
    }

    fn from_cbor_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(Self::from_slice(bytes)?)
    }
}

impl CborBytes for cashu::PublicKey {
    fn to_cbor_bytes(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn from_cbor_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(Self::from_slice(bytes)?)
    }
}

impl CborBytes for cashu::nut02::ShortKeysetId {
    fn to_cbor_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn from_cbor_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(Self::from_bytes(bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    const PUBKEY: &str = "02b463e1f803480e0964a1f65b508b77e2e5d1d3054e94ba1d353b9db76e453da5";

    fn assert_round_trip<T>(value: T, len: usize)
    where
        T: CborBytes + Clone + PartialEq + std::fmt::Debug,
    {
        let bytes = value.to_cbor_bytes();
        assert_eq!(bytes.len(), len);
        assert_eq!(T::from_cbor_bytes(&bytes).unwrap(), value);
        assert_ne!(T::from_cbor_bytes(&bytes[..len - 1]).ok(), Some(value));
    }

    #[test]
    fn cbor_bytes_round_trip() {
        assert_round_trip(bitcoin::secp256k1::PublicKey::from_str(PUBKEY).unwrap(), 33);
        assert_round_trip(cashu::PublicKey::from_hex(PUBKEY).unwrap(), 33);
        assert_round_trip(
            cashu::nut02::ShortKeysetId::from(cashu::Id::from_str("00ad268c4d1f5826").unwrap()),
            8,
        );
    }
}
