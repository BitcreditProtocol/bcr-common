use bitcoin::base64::engine::{GeneralPurpose, general_purpose};
use bitcoin::base64::{Engine as _, alphabet};
use bitcoin::secp256k1::PublicKey;
use cashu::{
    Amount, CurrencyUnit, KeySetInfo, MintUrl, Proof, Proofs,
    nut00::{Error, ProofV4, token::TokenV4Token},
    nut02::{self, ShortKeysetId},
    nuts::Id,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use crate::core::{ID_PREFIX, network_char, network_from_char};

/// Raw binary counterpart of [`ID_PREFIX`]
pub const RAW_PREFIX: &str = "braw";
/// Current token format version
pub const VERSION_V5: char = 'C';

#[doc(hidden)]
#[macro_export]
macro_rules! ensure_cdk {
    ($cond:expr, $err:expr) => {
        if !$cond {
            return Err($err);
        }
    };
}

/// Token Enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Token {
    BitcrV4(BitcrTokenV4),
    BitcrV5(BitcrTokenV5),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let token = match self {
            Self::BitcrV4(token) => token.to_string(),
            Self::BitcrV5(token) => token.to_string(),
        };

        write!(f, "{token}")
    }
}

impl Token {
    /// Create new bitcrV4 [`Token`], the legacy format
    pub fn new_bitcr(
        mint_url: MintUrl,
        proofs: Proofs,
        memo: Option<String>,
        unit: CurrencyUnit,
    ) -> Self {
        let proofs = proofs_to_tokenv4(proofs);

        Self::BitcrV4(BitcrTokenV4 {
            mint_url,
            unit,
            memo,
            token: proofs,
        })
    }

    /// Create new bitcrV5 [`Token`]
    pub fn new_bitcr_v5(
        mint_id: PublicKey,
        network: bitcoin::Network,
        mint_url: Option<String>,
        proofs: Proofs,
        memo: Option<String>,
        unit: CurrencyUnit,
    ) -> Self {
        let proofs = proofs_to_tokenv4(proofs);

        Self::BitcrV5(BitcrTokenV5 {
            network,
            mint_id,
            mint_url,
            unit,
            memo,
            token: proofs,
        })
    }

    /// Proofs in [`Token`]
    pub fn proofs(&self, mint_keysets: &[KeySetInfo]) -> Result<Proofs, Error> {
        match self {
            Self::BitcrV4(token) => token.proofs(mint_keysets),
            Self::BitcrV5(token) => token.proofs(mint_keysets),
        }
    }

    /// Total value of [`Token`]
    pub fn value(&self) -> Result<Amount, Error> {
        match self {
            Self::BitcrV4(token) => token.value(),
            Self::BitcrV5(token) => token.value(),
        }
    }

    /// [`Token`] memo
    pub fn memo(&self) -> &Option<String> {
        match self {
            Self::BitcrV4(token) => token.memo(),
            Self::BitcrV5(token) => token.memo(),
        }
    }

    /// Unit
    pub fn unit(&self) -> Option<CurrencyUnit> {
        match self {
            Self::BitcrV4(token) => Some(token.unit().clone()),
            Self::BitcrV5(token) => Some(token.unit().clone()),
        }
    }

    /// Mint url, a connectivity hint and not an identity
    pub fn mint_url(&self) -> Option<MintUrl> {
        match self {
            Self::BitcrV4(token) => Some(token.mint_url.clone()),
            Self::BitcrV5(token) => token.mint_url(),
        }
    }

    /// Clowder node id of the mint, the mint identity
    pub fn mint_id(&self) -> Option<PublicKey> {
        match self {
            Self::BitcrV4(_) => None,
            Self::BitcrV5(token) => Some(token.mint_id),
        }
    }

    /// Bitcoin network the token belongs to
    pub fn network(&self) -> Option<bitcoin::Network> {
        match self {
            Self::BitcrV4(_) => None,
            Self::BitcrV5(token) => Some(token.network),
        }
    }

    /// Serialize the token to raw binary
    pub fn to_raw_bytes(&self) -> Result<Vec<u8>, Error> {
        match self {
            Self::BitcrV4(token) => token.to_raw_bytes(),
            Self::BitcrV5(token) => token.to_raw_bytes(),
        }
    }
}

impl FromStr for Token {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(token) = BitcrTokenV5::from_str(s) {
            return Ok(Token::BitcrV5(token));
        }
        match BitcrTokenV4::from_str(s) {
            Ok(token) => Ok(Token::BitcrV4(token)),
            _ => Err(Error::UnsupportedToken),
        }
    }
}

impl TryFrom<&Vec<u8>> for Token {
    type Error = Error;

    fn try_from(bytes: &Vec<u8>) -> Result<Self, Self::Error> {
        if let Ok(token) = BitcrTokenV5::try_from(bytes) {
            return Ok(Token::BitcrV5(token));
        }
        if let Ok(token) = BitcrTokenV4::try_from(bytes) {
            return Ok(Token::BitcrV4(token));
        }
        Err(Error::UnsupportedToken)
    }
}

/// Token V4
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitcrTokenV4 {
    /// Mint Url
    #[serde(rename = "m")]
    pub mint_url: MintUrl,
    /// Token Unit
    #[serde(rename = "u")]
    pub unit: CurrencyUnit,
    /// Memo for token
    #[serde(rename = "d", skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    /// Proofs grouped by keyset_id
    #[serde(rename = "t")]
    pub token: Vec<TokenV4Token>,
}

impl BitcrTokenV4 {
    /// Proofs from token
    pub fn proofs(&self, mint_keysets: &[KeySetInfo]) -> Result<Proofs, Error> {
        let mut proofs: Proofs = vec![];
        for t in self.token.iter() {
            let long_id = Id::from_short_keyset_id(&t.keyset_id, mint_keysets)?;
            proofs.extend(t.proofs.iter().map(|p| p.into_proof(&long_id)));
        }
        Ok(proofs)
    }

    /// Value - errors if duplicate proofs are found
    #[inline]
    pub fn value(&self) -> Result<Amount, Error> {
        value_of(&self.token)
    }
    /// Memo
    #[inline]
    pub fn memo(&self) -> &Option<String> {
        &self.memo
    }

    /// Unit
    #[inline]
    pub fn unit(&self) -> &CurrencyUnit {
        &self.unit
    }

    /// Serialize the token to raw binary
    pub fn to_raw_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut prefix = b"brawB".to_vec();
        let mut data = Vec::new();
        ciborium::into_writer(self, &mut data).map_err(Error::CiboriumSerError)?;
        prefix.extend(data);
        Ok(prefix)
    }
}

impl fmt::Display for BitcrTokenV4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use serde::ser::Error;
        let mut data = Vec::new();
        ciborium::into_writer(self, &mut data).map_err(|e| fmt::Error::custom(e.to_string()))?;
        let encoded = general_purpose::URL_SAFE.encode(data);
        write!(f, "bitcrB{encoded}")
    }
}

impl FromStr for BitcrTokenV4 {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("bitcrB").ok_or(Error::UnsupportedToken)?;

        let decoded = decode_base64(s)?;
        let token: BitcrTokenV4 = ciborium::from_reader(&decoded[..])?;
        Ok(token)
    }
}

impl TryFrom<&Vec<u8>> for BitcrTokenV4 {
    type Error = Error;

    fn try_from(bytes: &Vec<u8>) -> Result<Self, Self::Error> {
        ensure_cdk!(bytes.len() >= 5, Error::UnsupportedToken);

        let prefix = String::from_utf8(bytes[..5].to_vec())?;
        ensure_cdk!(prefix.as_str() == "brawB", Error::UnsupportedToken);

        Ok(ciborium::from_reader(&bytes[5..])?)
    }
}

/// Token V5, identifies its mint by clowder node id
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitcrTokenV5 {
    /// Bitcoin network, carried in the string prefix
    pub network: bitcoin::Network,
    /// Clowder node id of the mint
    pub mint_id: PublicKey,
    /// Mint url, a connectivity hint and not an identity
    pub mint_url: Option<String>,
    /// Token Unit
    pub unit: CurrencyUnit,
    /// Memo for token
    pub memo: Option<String>,
    /// Proofs grouped by keyset_id
    pub token: Vec<TokenV4Token>,
}

/// Cbor payload of a V5 token, the network lives in the prefix
#[derive(Debug, Serialize, Deserialize)]
struct PayloadV5 {
    #[serde(
        rename = "k",
        serialize_with = "serialize_pubkey",
        deserialize_with = "deserialize_pubkey"
    )]
    mint_id: PublicKey,
    #[serde(rename = "m", default, skip_serializing_if = "Option::is_none")]
    mint_url: Option<String>,
    #[serde(rename = "u")]
    unit: CurrencyUnit,
    #[serde(rename = "d", default, skip_serializing_if = "Option::is_none")]
    memo: Option<String>,
    #[serde(rename = "t")]
    token: Vec<TokenV4Token>,
}

impl PayloadV5 {
    fn into_token(self, network: bitcoin::Network) -> BitcrTokenV5 {
        BitcrTokenV5 {
            network,
            mint_id: self.mint_id,
            mint_url: self.mint_url,
            unit: self.unit,
            memo: self.memo,
            token: self.token,
        }
    }
}

impl From<&BitcrTokenV5> for PayloadV5 {
    fn from(token: &BitcrTokenV5) -> Self {
        Self {
            mint_id: token.mint_id,
            mint_url: token.mint_url.clone(),
            unit: token.unit.clone(),
            memo: token.memo.clone(),
            token: token.token.clone(),
        }
    }
}

impl BitcrTokenV5 {
    /// Proofs from token
    pub fn proofs(&self, mint_keysets: &[KeySetInfo]) -> Result<Proofs, Error> {
        let mut proofs: Proofs = vec![];
        for t in self.token.iter() {
            let long_id = resolve_keyset_id(&t.keyset_id, mint_keysets)?;
            proofs.extend(t.proofs.iter().map(|p| p.into_proof(&long_id)));
        }
        Ok(proofs)
    }

    /// Value - errors if duplicate proofs are found
    #[inline]
    pub fn value(&self) -> Result<Amount, Error> {
        value_of(&self.token)
    }

    /// Memo
    #[inline]
    pub fn memo(&self) -> &Option<String> {
        &self.memo
    }

    /// Unit
    #[inline]
    pub fn unit(&self) -> &CurrencyUnit {
        &self.unit
    }

    /// Mint url hint, if it parses
    #[inline]
    pub fn mint_url(&self) -> Option<MintUrl> {
        self.mint_url
            .as_ref()
            .and_then(|url| MintUrl::from_str(url).ok())
    }

    /// Serialize the token to raw binary
    pub fn to_raw_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut prefix =
            format!("{RAW_PREFIX}{}{VERSION_V5}", network_char(&self.network)).into_bytes();
        let mut data = Vec::new();
        ciborium::into_writer(&PayloadV5::from(self), &mut data)
            .map_err(Error::CiboriumSerError)?;
        prefix.extend(data);
        Ok(prefix)
    }
}

impl fmt::Display for BitcrTokenV5 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use serde::ser::Error;
        let mut data = Vec::new();
        ciborium::into_writer(&PayloadV5::from(self), &mut data)
            .map_err(|e| fmt::Error::custom(e.to_string()))?;
        let encoded = general_purpose::URL_SAFE.encode(data);
        write!(
            f,
            "{ID_PREFIX}{}{VERSION_V5}{encoded}",
            network_char(&self.network)
        )
    }
}

impl FromStr for BitcrTokenV5 {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix(ID_PREFIX).ok_or(Error::UnsupportedToken)?;
        let network = s
            .chars()
            .next()
            .and_then(network_from_char)
            .ok_or(Error::UnsupportedToken)?;
        ensure_cdk!(
            s.chars().nth(1) == Some(VERSION_V5),
            Error::UnsupportedToken
        );

        let decoded = decode_base64(&s[2..])?;
        let payload: PayloadV5 = ciborium::from_reader(&decoded[..])?;
        Ok(payload.into_token(network))
    }
}

impl TryFrom<&Vec<u8>> for BitcrTokenV5 {
    type Error = Error;

    fn try_from(bytes: &Vec<u8>) -> Result<Self, Self::Error> {
        let header = RAW_PREFIX.len() + 2;
        ensure_cdk!(bytes.len() >= header, Error::UnsupportedToken);
        ensure_cdk!(
            &bytes[..RAW_PREFIX.len()] == RAW_PREFIX.as_bytes(),
            Error::UnsupportedToken
        );

        let network =
            network_from_char(bytes[RAW_PREFIX.len()] as char).ok_or(Error::UnsupportedToken)?;
        ensure_cdk!(
            bytes[RAW_PREFIX.len() + 1] == VERSION_V5 as u8,
            Error::UnsupportedToken
        );

        let payload: PayloadV5 = ciborium::from_reader(&bytes[header..])?;
        Ok(payload.into_token(network))
    }
}

fn serialize_pubkey<S>(pubkey: &PublicKey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_bytes(&pubkey.serialize())
}

fn deserialize_pubkey<'de, D>(deserializer: D) -> Result<PublicKey, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let bytes = Vec::<u8>::deserialize(deserializer)?;
    PublicKey::from_slice(&bytes).map_err(serde::de::Error::custom)
}

fn decode_base64(s: &str) -> Result<Vec<u8>, Error> {
    let decode_config = general_purpose::GeneralPurposeConfig::new()
        .with_decode_padding_mode(bitcoin::base64::engine::DecodePaddingMode::Indifferent);
    Ok(GeneralPurpose::new(&alphabet::URL_SAFE, decode_config).decode(s)?)
}

/// Expands a short keyset id, rejecting it when it matches more than one keyset
fn resolve_keyset_id(short_id: &ShortKeysetId, mint_keysets: &[KeySetInfo]) -> Result<Id, Error> {
    let long_id = Id::from_short_keyset_id(short_id, mint_keysets)?;
    let matches = mint_keysets
        .iter()
        .filter(|keyset| ShortKeysetId::from(keyset.id) == *short_id)
        .count();
    ensure_cdk!(matches <= 1, Error::NUT02(nut02::Error::IncorrectKeysetId));
    Ok(long_id)
}

fn value_of(tokens: &[TokenV4Token]) -> Result<Amount, Error> {
    let proofs: Vec<&ProofV4> = tokens.iter().flat_map(|t| &t.proofs).collect();
    let unique_count = proofs
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len();

    // Check if there are any duplicate proofs
    if unique_count != proofs.len() {
        return Err(Error::DuplicateProofs);
    }

    Ok(Amount::try_sum(
        tokens
            .iter()
            .map(|t| Amount::try_sum(t.proofs.iter().map(|p| p.amount)))
            .collect::<Result<Vec<Amount>, _>>()?,
    )?)
}

fn proofs_to_tokenv4(proofs: Proofs) -> Vec<TokenV4Token> {
    proofs
        .into_iter()
        .fold(HashMap::new(), |mut acc, val| {
            acc.entry(val.keyset_id)
                .and_modify(|p: &mut Vec<Proof>| p.push(val.clone()))
                .or_insert(vec![val.clone()]);
            acc
        })
        .into_iter()
        .map(|(id, proofs)| TokenV4Token::new(id, proofs))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cashu::nut02 as cdk02;
    use std::str::FromStr;

    #[test]
    fn test_token_str_round_trip() {
        let token_str = "bitcrBpGFtdWh0dHA6Ly9sb2NhbGhvc3Q6MzMzOGF1Y3NhdGF0gaJhaUgArSaMTR9YJmFwgaNhYQFhc3hAOWE2ZGJiODQ3YmQyMzJiYTc2ZGIwZGYxOTcyMTZiMjlkM2I4Y2MxNDU1M2NkMjc4MjdmYzFjYzk0MmZlZGI0ZWFjWCEDhhhUP_trhpXfStS6vN6So0qWvc2X3O4NfM-Y1HISZ5JhZGlUaGFuayB5b3U";

        let token = Token::from_str(token_str).unwrap();
        assert!(matches!(token, Token::BitcrV4(_)));
        let Token::BitcrV4(inner) = token.clone() else {
            panic!("expected a V4 token");
        };
        assert_eq!(inner.token.len(), 1);
        assert_eq!(inner.token[0].keyset_id.to_string(), "00ad268c4d1f5826");

        token.to_string().strip_prefix("bitcrB").unwrap();
        assert_eq!(inner.mint_url.to_string(), "http://localhost:3338");
        assert_eq!(
            inner.token[0].keyset_id,
            cdk02::ShortKeysetId::from_str("00ad268c4d1f5826").unwrap()
        );
        assert_eq!(inner.unit.clone(), cashu::CurrencyUnit::Sat);

        let encoded = &inner.to_string();

        let token_data = BitcrTokenV4::from_str(encoded).unwrap();
        assert_eq!(token_data, inner);
    }

    #[test]
    fn incorrect_tokens() {
        let incorrect_prefix = "casshuAeyJ0b2tlbiI6W3sibWludCI6Imh0dHBzOi8vODMzMy5zcGFjZTozMzM4IiwicHJvb2ZzIjpbeyJhbW91bnQiOjIsImlkIjoiMDA5YTFmMjkzMjUzZTQxZSIsInNlY3JldCI6IjQwNzkxNWJjMjEyYmU2MWE3N2UzZTZkMmFlYjRjNzI3OTgwYmRhNTFjZDA2YTZhZmMyOWUyODYxNzY4YTc4MzciLCJDIjoiMDJiYzkwOTc5OTdkODFhZmIyY2M3MzQ2YjVlNDM0NWE5MzQ2YmQyYTUwNmViNzk1ODU5OGE3MmYwY2Y4NTE2M2VhIn0seyJhbW91bnQiOjgsImlkIjoiMDA5YTFmMjkzMjUzZTQxZSIsInNlY3JldCI6ImZlMTUxMDkzMTRlNjFkNzc1NmIwZjhlZTBmMjNhNjI0YWNhYTNmNGUwNDJmNjE0MzNjNzI4YzcwNTdiOTMxYmUiLCJDIjoiMDI5ZThlNTA1MGI4OTBhN2Q2YzA5NjhkYjE2YmMxZDVkNWZhMDQwZWExZGUyODRmNmVjNjlkNjEyOTlmNjcxMDU5In1dfV0sInVuaXQiOiJzYXQiLCJtZW1vIjoiVGhhbmsgeW91LiJ9";

        let incorrect_prefix_token = Token::from_str(incorrect_prefix);

        assert!(incorrect_prefix_token.is_err());

        let no_prefix = "eyJ0b2tlbiI6W3sibWludCI6Imh0dHBzOi8vODMzMy5zcGFjZTozMzM4IiwicHJvb2ZzIjpbeyJhbW91bnQiOjIsImlkIjoiMDA5YTFmMjkzMjUzZTQxZSIsInNlY3JldCI6IjQwNzkxNWJjMjEyYmU2MWE3N2UzZTZkMmFlYjRjNzI3OTgwYmRhNTFjZDA2YTZhZmMyOWUyODYxNzY4YTc4MzciLCJDIjoiMDJiYzkwOTc5OTdkODFhZmIyY2M3MzQ2YjVlNDM0NWE5MzQ2YmQyYTUwNmViNzk1ODU5OGE3MmYwY2Y4NTE2M2VhIn0seyJhbW91bnQiOjgsImlkIjoiMDA5YTFmMjkzMjUzZTQxZSIsInNlY3JldCI6ImZlMTUxMDkzMTRlNjFkNzc1NmIwZjhlZTBmMjNhNjI0YWNhYTNmNGUwNDJmNjE0MzNjNzI4YzcwNTdiOTMxYmUiLCJDIjoiMDI5ZThlNTA1MGI4OTBhN2Q2YzA5NjhkYjE2YmMxZDVkNWZhMDQwZWExZGUyODRmNmVjNjlkNjEyOTlmNjcxMDU5In1dfV0sInVuaXQiOiJzYXQiLCJtZW1vIjoiVGhhbmsgeW91LiJ9";

        let no_prefix_token = Token::from_str(no_prefix);

        assert!(no_prefix_token.is_err());

        let correct_token = "bitcrBo2F0gqJhaUgA_9SLj17PgGFwgaNhYQFhc3hAYWNjMTI0MzVlN2I4NDg0YzNjZjE4NTAxNDkyMThhZjkwZjcxNmE1MmJmNGE1ZWQzNDdlNDhlY2MxM2Y3NzM4OGFjWCECRFODGd5IXVW-07KaZCvuWHk3WrnnpiDhHki6SCQh88-iYWlIAK0mjE0fWCZhcIKjYWECYXN4QDEzMjNkM2Q0NzA3YTU4YWQyZTIzYWRhNGU5ZjFmNDlmNWE1YjRhYzdiNzA4ZWIwZDYxZjczOGY0ODMwN2U4ZWVhY1ghAjRWqhENhLSsdHrr2Cw7AFrKUL9Ffr1XN6RBT6w659lNo2FhAWFzeEA1NmJjYmNiYjdjYzY0MDZiM2ZhNWQ1N2QyMTc0ZjRlZmY4YjQ0MDJiMTc2OTI2ZDNhNTdkM2MzZGNiYjU5ZDU3YWNYIQJzEpxXGeWZN5qXSmJjY8MzxWyvwObQGr5G1YCCgHicY2FtdWh0dHA6Ly9sb2NhbGhvc3Q6MzMzOGF1Y3NhdA==";

        let correct_token = Token::from_str(correct_token);

        assert!(correct_token.is_ok());
    }

    #[test]
    fn test_token_value() {
        let token_str = "bitcrBpGFteC9odHRwczovL21pbnQud2lsZGNhdDAuY2xvd2Rlci1kZXYubWluaWJpbGwudGVjaGF1Y3NhdGFkY3NhdGF0gaJhaUgBRjxG54nx4WFwh6RhYQRhc3hAYjdkNTUwZjljZjYyYzk0NmM3YzdjNDVkNjc4ZmUxMzc3OTdkNWRkMDMwZjQxMWY1NDg2OTU4MjE1MjRkYWY0M2FjWCECy1iLQBVPbznqF_cuQ_hj7sVg39ZGE-4aFvTnkPPIyMhhZKNhZVggmwzCLAxNKeNsJw_ZM-n1nfyE1bQSpXB9rvE8sLjGK_xhc1ggmYxKXPHd1N8yIRYn7KCa6Jl28EMVsQF-QwJHxXHe34thclggXUw1Z18UJiVzDdi0soIgil4JGA6iBRBHDeK_Qy1tYFSkYWEYgGFzeEAxMmJiNjg2YTAwNTg5N2MyYmRkZTBkODQxMzE0Mjg2NmIzMzkwZWI5Zjk5NzYxYmQ2NzVhYzI3YmY3NTFiZWI3YWNYIQP_o6JSVmMSfJT5I6fDZH7iZUXsLxmthXXtlSA8oVI3gGFko2FlWCCPbRLKnGTc1_WD_a91Fg-rr8P8cYk2gTZ8Qk19HJYWwWFzWCBLUfhonMlxUp4VVi-nxr-vDPag-o2ZZx0c-W7d3OSlHGFyWCCWjjuYWMj05LXrMebjwod7LWDYEVbskxEvAghk_GvCxqRhYQhhc3hAOTY0ZGYwOWQwOGQ5ZDRjY2E0YmVmYmM4ZWUzYjA4YWM5NTQ1ZjcxOTAwYWNkYzIzZWIzZDExZjUwNzIxZTFlN2FjWCEDRsrtPoGEFuuwRPhNSDKMGCZSuhPNQjh6bzuq_LTOrKdhZKNhZVggtp564XxyxpkRS_TirpegGZHyWXm4v4OAUTmzLO9ktnZhc1gg4fA_a2epcYSprnk4WAIyFNwkvuJCRsRvGEKpDHvw8RNhclggcLgcKktVlCP0ILVz53sP6tWK5Zn2p_0maxSMlD0-3jakYWEYQGFzeEA4NmJlNDBlNzRlOGVjYjc3YzA4NTdmYWEzMWY4MGQwZTQ1YzNlZjEzNzNjMTBjZWI1NWQzMzFhY2FhOWE5NjA0YWNYIQPDD9NSpAzTe8Aii_xweeyw2oa9nA5zz2CR4QdlYcr8JmFko2FlWCCuy6WOHlp4S-UHUIkQjaa4sNw8zkLBCSv-DMl8X6ASTWFzWCBvrg90GrXfBzLnWg4E0T8CA0jA5hVuHAVrLKZ9kLteq2FyWCCIP-6v7_r1nO1WblfP-Mkf6D_hL7ELH7HyzkURBxEhCKRhYQFhc3hAZmI3YjEwMzFiMjRkNDBjODBjZTA1MjNkNGM0YzQxYTQ3MWY4OGI3ZWIzYTk1OWVlZDA4ZjQ0YWVjYjdhNThjNGFjWCEDcX4YKFg8E2RWOMbJzWfm-f-0mlg89Cuy0ZFDYSB5-a1hZKNhZVggQJ2jupPgw1S-NMqPdCTq8OkhjOsolRfBN5unMlWlQTZhc1ggAssEObaR2OZEwXj01Q7GMQ4xfaENrggLxU48minMRtVhclgg2SjONoi2MlTIF9zUhXAbudms-8CLSpI8cVwl_f23dRCkYWEZAgBhc3hAODY1NjNkN2RjNzYzYjZkN2MxYjQ4MWJlMWIwMzY5OTRhNjFlMGZmNDU2MWZmZGJlMzQ1ZDc4NjM5ZTJmM2M2OGFjWCEDmaQqK0Ng1ou8FFpzezyjYuJ3VfvAqh_fURHANFya7yxhZKNhZVggCP8wUozvBK7B89fpGmkLC4dXGJbaf919pnND5zXAo1Fhc1ggAMn3Zb1qlMCUQhpsiSnBmFQDBUa8RCEuGxWamPSTfKlhclggjzGTRZcZfx6V7b3Qds7p-q-yipRB2x2DVG2Y2rGmu1qkYWEZAQBhc3hANDY3MDM3OTgxNGMzM2Y3OTRmNTdhOTdjNTQ5ZTFmZDE1YTQyYTE4NWVjZTM2MGFmN2ZkYjliMTYxOTMxN2YxZmFjWCED9QgfGg6NNmI27aVQTGIGOD_AOvCv6ULNk8Gls3gb-YthZKNhZVggXGVixAC9eS6SITqMI2eM6yDdFbXR6idgIUXBkZA9qxthc1ggrb-pKZrAjxBl-BI-IwEvzvChGoSdFFyKMNhCGkB8h0JhclggKyY-ukPdK5o3RcsOGcM44L3KnG_lG4z2CupyqkvWqLk=";

        let token = Token::from_str(token_str).unwrap();
        token.to_string().strip_prefix("bitcrB").unwrap();
        let Token::BitcrV4(token) = token else {
            panic!("expected a V4 token");
        };
        assert_eq!(token.value().unwrap(), cashu::Amount::from(973));
        assert_eq!(token.unit.to_string(), "sat");
    }

    #[test]
    fn test_serde() {
        let token_str = "bitcrBo2FtdWh0dHA6Ly9sb2NhbGhvc3Q6NDM0M2F1ZWNyc2F0YXSBomFpSABp3j5af6uYYXCHpGFhGEBhc3hAODcyYmIxNzY0ODA3NDY2YWUxMDY2MGQxMjA5ODUxYzQ2MGJmZjJmNDZiY2YyZmJmM2QzY2NjY2QyYzllMzNiMGFjWCECgISwm2AJEFh3vxZKCNjnxx3pZ8BBav7a5AXLtMVQVjRhZPakYWEYgGFzeEBhY2QzYzI5YjlhZjEwYmM4MTdiOWUxNGFhMjllZjIxODkzYmZjZWMwMzFmYWQyM2IxOWExMDhjMzFhZmQyODMyYWNYIQIMmOnUpdbYTBtRceuCXy_qajysL6sG9CsvtRSBukjWO2Fk9qRhYRkCAGFzeEA4ZmU1NDNmOTMxYjA4MzhhOTA3NmMyMjljNzg1OWU3MTc0MTUzMGVmMGFiZWMyMzlkOWE0ZWNjOGEyMGNlYzRmYWNYIQPqj23wVNNNx42KP28By2a5i6N5TMkVU8lixcZ3aeiA7WFk9qRhYQRhc3hAMzk4YjYzMmU4MTZmNzQ4Njc1N2E3NTk5Mzc2YjlhYmFkMGFmNGQwMTVkYTQ0Mjk5Zjg2OGYxNWM4ODdmNDNjYmFjWCEDo8X2Y4JoRJ1hGSXDSVgQH-YXpFw_NYXtPIUv5xJcX-9hZPakYWEIYXN4QGJjNjM4NTYxN2Q2NjJkN2Q5NWIxNDBlMTU4Y2MzMTYwZjAzMmQxMWJiZGEzZWY3MDRhYzcyOTliM2EzYjQyOThhY1ghA_UAeY1dWx5QHqsvepcUK68xfHZJIbuRCaM45uN4t9vsYWT2pGFhGQEAYXN4QDFlNGQ1ZGI1MTc2MzU2YWEwZTI2MzJmZDlkYTUxMjYzYmY1M2EyMjFkNmNhZmE5Y2U4YTExMjg4MGNhMWQwZmZhY1ghAm3brXrx4F8HY8-YeC-msEuI9vfSzBKayKzab58A6xYwYWT2pGFhAWFzeEAwNzcyNTMyYTJkMjZkNDcyOTZjNzQ3NzMxN2NhZjQzOTdjZjA4MmM0ZjkwMzE4YWJjMDljZGRmZTEyMzFiYThlYWNYIQPeNBo_DX-qSXr52rqbwhGKWx9VNpaddKwORBP9-43JzmFk9g==";

        let token = Token::from_str(token_str).unwrap();
        let token_json = serde_json::to_string(&token).unwrap();
        let deserialized_token: Token = serde_json::from_str(&token_json).unwrap();
        assert_eq!(token, deserialized_token);
    }

    const MINT_ID: &str = "02b463e1f803480e0964a1f65b508b77e2e5d1d3054e94ba1d353b9db76e453da5";
    const MAINNET_V5: &str = "bitcrmCpGFrWCECtGPh-ANIDglkofZbUIt34uXR0wVOlLodNTudt25FPaVhbXVodHRwOi8vbG9jYWxob3N0OjQzNDNhdWVjcnNhdGF0gaJhaUgA_9SLj17PgGFwgaNhYQFhc3hAYWNjMTI0MzVlN2I4NDg0YzNjZjE4NTAxNDkyMThhZjkwZjcxNmE1MmJmNGE1ZWQzNDdlNDhlY2MxM2Y3NzM4OGFjWCECRFODGd5IXVW-07KaZCvuWHk3WrnnpiDhHki6SCQh888=";
    const REGTEST_EMPTY_V5: &str =
        "bitcrrCo2FrWCECtGPh-ANIDglkofZbUIt34uXR0wVOlLodNTudt25FPaVhdWNzYXRhdIA=";

    fn v5(token_str: &str) -> BitcrTokenV5 {
        let Token::BitcrV5(token) = Token::from_str(token_str).unwrap() else {
            panic!("expected a V5 token");
        };
        token
    }

    fn keyset_info(id: Id) -> KeySetInfo {
        KeySetInfo {
            id,
            unit: cashu::CurrencyUnit::Sat,
            active: true,
            input_fee_ppk: 0,
            final_expiry: None,
        }
    }

    #[test]
    fn test_token_v5_spec_vectors() {
        let mainnet = v5(MAINNET_V5);
        assert_eq!(mainnet.network, bitcoin::Network::Bitcoin);
        assert_eq!(mainnet.mint_id, PublicKey::from_str(MINT_ID).unwrap());
        assert_eq!(
            mainnet.mint_url().unwrap().to_string(),
            "http://localhost:4343"
        );
        assert_eq!(mainnet.unit, cashu::CurrencyUnit::Custom("crsat".into()));
        assert_eq!(mainnet.memo, None);
        assert_eq!(mainnet.token.len(), 1);
        assert_eq!(mainnet.token[0].keyset_id.to_string(), "00ffd48b8f5ecf80");
        assert_eq!(mainnet.value().unwrap(), cashu::Amount::from(1));
        // cashu's ProofV4 always emits the proof level dleq key, so re-encoding a
        // proof carrying token differs from the spec example by an explicit d null
        assert_eq!(v5(&mainnet.to_string()), mainnet);

        let regtest = v5(REGTEST_EMPTY_V5);
        assert_eq!(regtest.network, bitcoin::Network::Regtest);
        assert_eq!(regtest.mint_id, PublicKey::from_str(MINT_ID).unwrap());
        assert_eq!(regtest.mint_url, None);
        assert_eq!(regtest.unit, cashu::CurrencyUnit::Sat);
        assert!(regtest.token.is_empty());
        assert_eq!(regtest.to_string(), REGTEST_EMPTY_V5);
    }

    #[test]
    fn test_token_v5_round_trip_every_network() {
        let mainnet = v5(MAINNET_V5);

        for (network, network_char) in [
            (bitcoin::Network::Bitcoin, 'm'),
            (bitcoin::Network::Testnet, 't'),
            (bitcoin::Network::Testnet4, 'T'),
            (bitcoin::Network::Signet, 's'),
            (bitcoin::Network::Regtest, 'r'),
        ] {
            let token = BitcrTokenV5 {
                network,
                ..mainnet.clone()
            };

            let encoded = token.to_string();
            assert!(encoded.starts_with(&format!("bitcr{network_char}C")));
            assert_eq!(v5(&encoded), token);

            let raw = token.to_raw_bytes().unwrap();
            assert_eq!(&raw[..6], format!("braw{network_char}C").as_bytes());
            assert_eq!(
                Token::try_from(&raw).unwrap(),
                Token::BitcrV5(token.clone())
            );
        }
    }

    #[test]
    fn test_token_v4_raw_bytes_round_trip() {
        let legacy = "bitcrBo2FtdWh0dHA6Ly9sb2NhbGhvc3Q6MzMzOGF1Y3NhdGF0gaJhaUgArSaMTR9YJmFwgaNhYQFhc3hAOWE2ZGJiODQ3YmQyMzJiYTc2ZGIwZGYxOTcyMTZiMjlkM2I4Y2MxNDU1M2NkMjc4MjdmYzFjYzk0MmZlZGI0ZWFjWCEDhhhUP_trhpXfStS6vN6So0qWvc2X3O4NfM-Y1HISZ5I=";
        let token = Token::from_str(legacy).unwrap();
        assert!(matches!(token, Token::BitcrV4(_)));
        let raw = token.to_raw_bytes().unwrap();
        assert_eq!(&raw[..5], b"brawB");
        assert_eq!(Token::try_from(&raw).unwrap(), token);
    }

    #[test]
    fn test_token_v5_rejects_unknown_network_and_version() {
        // deprecated version A, unknown version, unknown network character
        for token_str in [
            MAINNET_V5.replace("bitcrmC", "bitcrmA"),
            MAINNET_V5.replace("bitcrmC", "bitcrmD"),
            MAINNET_V5.replace("bitcrmC", "bitcrxC"),
            MAINNET_V5.replace("bitcrmC", "bitcr"),
        ] {
            assert!(Token::from_str(&token_str).is_err(), "{token_str}");
        }
    }

    #[test]
    fn test_token_v5_ignores_unknown_keys_and_explicit_nulls() {
        let decoded = decode_base64(&MAINNET_V5["bitcrmC".len()..]).unwrap();
        let payload: ciborium::Value = ciborium::from_reader(&decoded[..]).unwrap();
        let mut map = payload.into_map().expect("cbor map");

        for entry in map.iter_mut() {
            if entry.0 == ciborium::Value::Text("m".into()) {
                entry.1 = ciborium::Value::Null;
            }
        }
        map.push((ciborium::Value::Text("d".into()), ciborium::Value::Null));
        map.push((
            ciborium::Value::Text("zz".into()),
            ciborium::Value::Text("from a future version".into()),
        ));

        let mut bytes = Vec::new();
        ciborium::into_writer(&ciborium::Value::Map(map), &mut bytes).unwrap();
        let token_str = format!("bitcrmC{}", general_purpose::URL_SAFE.encode(bytes));

        let token = v5(&token_str);
        assert_eq!(token.mint_url, None);
        assert_eq!(token.memo, None);
        assert_eq!(token.mint_id, PublicKey::from_str(MINT_ID).unwrap());
    }

    #[test]
    fn test_token_v5_padding_is_optional() {
        let padded = MAINNET_V5;
        let unpadded = padded.trim_end_matches('=');
        assert_ne!(padded, unpadded);
        assert_eq!(v5(padded), v5(unpadded));
    }

    #[test]
    fn test_token_v5_rejects_ambiguous_short_keyset_id() {
        let mainnet = v5(MAINNET_V5);
        let first = Id::from_str(&format!("01aabbccddeeff00{}", "11".repeat(25))).unwrap();
        let second = Id::from_str(&format!("01aabbccddeeff00{}", "22".repeat(25))).unwrap();
        assert_eq!(
            cdk02::ShortKeysetId::from(first),
            cdk02::ShortKeysetId::from(second)
        );

        let token = BitcrTokenV5 {
            token: vec![TokenV4Token {
                keyset_id: cdk02::ShortKeysetId::from(first),
                proofs: mainnet.token[0].proofs.clone(),
            }],
            ..mainnet
        };

        assert!(token.proofs(&[keyset_info(first)]).is_ok());
        assert!(
            token
                .proofs(&[keyset_info(first), keyset_info(second)])
                .is_err()
        );
    }
}
