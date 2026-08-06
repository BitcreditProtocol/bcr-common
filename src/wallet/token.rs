// ----- standard library imports
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::str::FromStr;
// ----- extra library imports
use bitcoin::base64::engine::{GeneralPurpose, general_purpose};
use bitcoin::base64::{Engine as _, alphabet};
use cashu::{Amount, CurrencyUnit, Id, MintUrl, nut02::ShortKeysetId};
use serde::{Deserialize, Serialize};
// ----- local modules
use crate::core::{ID_PREFIX, NodeId, network_char, network_from_char};
use crate::ecash::{KeySetInfo, Proofs};
use crate::wallet::proof::TokenV4Token;
use crate::wallet::{Error, Result};

// ----- end imports

/// Raw binary counterpart of [`ID_PREFIX`]
pub const RAW_PREFIX: &str = "braw";
/// Legacy token format version, kept for tokens already in circulation
pub const VERSION_V4: char = 'B';
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

/// The chars that follow [`ID_PREFIX`]/[`RAW_PREFIX`] and select the layout of the
/// rest of the token: a bare `<version>` for the legacy V4 form, `<network><version>`
/// from V5 onwards. Single source of truth for both parsing and rendering.
///
/// V4 puts a version char exactly where later versions put a network char, so the
/// two alphabets must stay disjoint, see `version_and_network_chars_are_disjoint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Header {
    V4,
    V5(bitcoin::Network),
}

impl Header {
    /// Bytes this header occupies after the prefix
    const fn len(&self) -> usize {
        match self {
            Self::V4 => 1,
            Self::V5(_) => 2,
        }
    }

    /// Reads the header from the bytes that follow the prefix
    fn parse(rest: &[u8]) -> Result<Self> {
        let first = *rest.first().ok_or(Error::UnsupportedToken)? as char;
        if first == VERSION_V4 {
            return Ok(Self::V4);
        }
        let network = network_from_char(first).ok_or(Error::UnknownNetwork(first))?;
        let version = *rest.get(1).ok_or(Error::UnsupportedToken)? as char;
        ensure_cdk!(version == VERSION_V5, Error::UnknownVersion(version));
        Ok(Self::V5(network))
    }
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V4 => write!(f, "{VERSION_V4}"),
            Self::V5(network) => write!(f, "{}{VERSION_V5}", network_char(network)),
        }
    }
}

/// A concrete token format, reduced to the two things that encoding needs
trait Encode {
    /// Header selecting this format
    fn header(&self) -> Header;
    /// Cbor body that follows the header
    fn body(&self) -> Result<Vec<u8>>;
}

/// Token Enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Token {
    BitcrV4(BitcrTokenV4),
    BitcrV5(BitcrTokenV5),
}

impl Token {
    /// Create new bitcrV4 [`Token`], the legacy format
    pub fn new_bitcr(
        mint_url: MintUrl,
        proofs: Proofs,
        memo: Option<String>,
        unit: CurrencyUnit,
    ) -> Self {
        Self::BitcrV4(BitcrTokenV4 {
            mint_url,
            unit,
            memo,
            token: group_proofs(proofs),
        })
    }

    /// Proofs in [`Token`]
    pub fn proofs(&self, mint_keysets: &[KeySetInfo]) -> Result<Proofs> {
        match self {
            Self::BitcrV4(token) => token.proofs(mint_keysets),
            Self::BitcrV5(token) => token.proofs(mint_keysets),
        }
    }

    /// Total value of [`Token`]
    pub fn value(&self) -> Result<Amount> {
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

    /// Mint url, a connectivity hint and not an identity. `None` when the token
    /// carries no hint or one that does not parse, see [`BitcrTokenV5::mint_url`]
    pub fn mint_url(&self) -> Option<MintUrl> {
        match self {
            Self::BitcrV4(token) => Some(token.mint_url.clone()),
            Self::BitcrV5(token) => token.mint_url(),
        }
    }

    /// Clowder node id of the mint, the mint identity. Absent from the legacy V4
    /// format, which only carries a mint url
    pub fn mint_id(&self) -> Option<&NodeId> {
        match self {
            Self::BitcrV4(_) => None,
            Self::BitcrV5(token) => Some(&token.mint_id),
        }
    }

    /// Proofs as carried, grouped by short keyset id. Needs no keysets, unlike
    /// [`Token::proofs`]
    pub fn groups(&self) -> &[TokenV4Token] {
        match self {
            Self::BitcrV4(token) => &token.token,
            Self::BitcrV5(token) => &token.token,
        }
    }

    /// Bitcoin network the token belongs to, absent from the legacy V4 format
    pub fn network(&self) -> Option<bitcoin::Network> {
        match self {
            Self::BitcrV4(_) => None,
            Self::BitcrV5(token) => Some(token.mint_id.network()),
        }
    }

    /// Serialize the token to raw binary
    pub fn to_raw_bytes(&self) -> Result<Vec<u8>> {
        to_raw_bytes(self)
    }

    /// Decodes the cbor body that follows a parsed [`Header`]
    fn decode(header: Header, body: &[u8]) -> Result<Self> {
        Ok(match header {
            Header::V4 => Self::BitcrV4(ciborium::from_reader(body)?),
            Header::V5(network) => {
                Self::BitcrV5(ciborium::from_reader::<PayloadV5, _>(body)?.into_token(network))
            }
        })
    }
}

impl Encode for Token {
    fn header(&self) -> Header {
        match self {
            Self::BitcrV4(token) => token.header(),
            Self::BitcrV5(token) => token.header(),
        }
    }

    fn body(&self) -> Result<Vec<u8>> {
        match self {
            Self::BitcrV4(token) => token.body(),
            Self::BitcrV5(token) => token.body(),
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_token(self, f)
    }
}

impl FromStr for Token {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let rest = s.strip_prefix(ID_PREFIX).ok_or(Error::UnsupportedToken)?;
        let header = Header::parse(rest.as_bytes())?;
        Self::decode(header, &decode_base64(&rest[header.len()..])?)
    }
}

impl TryFrom<&Vec<u8>> for Token {
    type Error = Error;

    fn try_from(bytes: &Vec<u8>) -> Result<Self> {
        let rest = bytes
            .strip_prefix(RAW_PREFIX.as_bytes())
            .ok_or(Error::UnsupportedToken)?;
        let header = Header::parse(rest)?;
        Self::decode(header, &rest[header.len()..])
    }
}

impl From<BitcrTokenV4> for Token {
    fn from(token: BitcrTokenV4) -> Self {
        Self::BitcrV4(token)
    }
}

impl From<BitcrTokenV5> for Token {
    fn from(token: BitcrTokenV5) -> Self {
        Self::BitcrV5(token)
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
    pub fn proofs(&self, mint_keysets: &[KeySetInfo]) -> Result<Proofs> {
        proofs_of(&self.token, mint_keysets)
    }

    /// Value - errors if duplicate proofs are found
    #[inline]
    pub fn value(&self) -> Result<Amount> {
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
    pub fn to_raw_bytes(&self) -> Result<Vec<u8>> {
        to_raw_bytes(self)
    }
}

impl Encode for BitcrTokenV4 {
    fn header(&self) -> Header {
        Header::V4
    }

    fn body(&self) -> Result<Vec<u8>> {
        encode_body(self)
    }
}

impl fmt::Display for BitcrTokenV4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_token(self, f)
    }
}

/// Token V5, identifies its mint by clowder node id
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitcrTokenV5 {
    /// Clowder node id of the mint: its identity together with the bitcoin
    /// network it runs on. The network half is what the token prefix carries, so
    /// prefix and mint identity cannot disagree
    pub mint_id: NodeId,
    /// Token Unit
    pub unit: CurrencyUnit,
    /// Mint url, a connectivity hint and not an identity
    pub mint_url: Option<String>,
    /// Memo for token
    pub memo: Option<String>,
    /// Proofs grouped by keyset_id
    pub token: Vec<TokenV4Token>,
}

impl BitcrTokenV5 {
    /// Create new bitcrV5 token holding `proofs`
    pub fn new(mint_id: NodeId, unit: CurrencyUnit, proofs: Proofs) -> Self {
        Self {
            mint_id,
            unit,
            mint_url: None,
            memo: None,
            token: group_proofs(proofs),
        }
    }

    /// Attach a mint url connectivity hint
    pub fn with_mint_url(mut self, mint_url: impl Into<String>) -> Self {
        self.mint_url = Some(mint_url.into());
        self
    }

    /// Attach a memo
    pub fn with_memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }

    /// Proofs from token
    pub fn proofs(&self, mint_keysets: &[KeySetInfo]) -> Result<Proofs> {
        proofs_of(&self.token, mint_keysets)
    }

    /// Value - errors if duplicate proofs are found
    #[inline]
    pub fn value(&self) -> Result<Amount> {
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

    /// Mint url hint, if it parses. A hint that does not parse is indistinguishable
    /// from an absent one here, read the `mint_url` field for the raw value
    #[inline]
    pub fn mint_url(&self) -> Option<MintUrl> {
        self.mint_url
            .as_ref()
            .and_then(|url| MintUrl::from_str(url).ok())
    }

    /// Serialize the token to raw binary
    pub fn to_raw_bytes(&self) -> Result<Vec<u8>> {
        to_raw_bytes(self)
    }
}

impl Encode for BitcrTokenV5 {
    fn header(&self) -> Header {
        Header::V5(self.mint_id.network())
    }

    fn body(&self) -> Result<Vec<u8>> {
        encode_body(&PayloadV5::from(self))
    }
}

impl fmt::Display for BitcrTokenV5 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_token(self, f)
    }
}

/// Cbor payload of a V5 token, the mint's network lives in the prefix
#[derive(Debug, Serialize, Deserialize)]
struct PayloadV5 {
    #[serde(rename = "k", with = "crate::wallet::cbor")]
    mint_key: bitcoin::secp256k1::PublicKey,
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
            mint_id: NodeId::new(self.mint_key, network),
            unit: self.unit,
            mint_url: self.mint_url,
            memo: self.memo,
            token: self.token,
        }
    }
}

impl From<&BitcrTokenV5> for PayloadV5 {
    fn from(token: &BitcrTokenV5) -> Self {
        Self {
            mint_key: token.mint_id.pub_key(),
            mint_url: token.mint_url.clone(),
            unit: token.unit.clone(),
            memo: token.memo.clone(),
            token: token.token.clone(),
        }
    }
}

fn encode_body<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    ciborium::into_writer(value, &mut body)?;
    Ok(body)
}

fn to_raw_bytes<T: Encode>(token: &T) -> Result<Vec<u8>> {
    let mut bytes = format!("{RAW_PREFIX}{}", token.header()).into_bytes();
    bytes.extend(token.body()?);
    Ok(bytes)
}

fn fmt_token<T: Encode>(token: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    use serde::ser::Error;
    let body = token
        .body()
        .map_err(|e| fmt::Error::custom(e.to_string()))?;
    write!(
        f,
        "{ID_PREFIX}{}{}",
        token.header(),
        general_purpose::URL_SAFE.encode(body)
    )
}

fn decode_base64(s: &str) -> Result<Vec<u8>> {
    let decode_config = general_purpose::GeneralPurposeConfig::new()
        .with_decode_padding_mode(bitcoin::base64::engine::DecodePaddingMode::Indifferent);
    Ok(GeneralPurpose::new(&alphabet::URL_SAFE, decode_config).decode(s)?)
}

/// Expands a short keyset id: it only identifies a keyset when exactly one of the
/// mint's advertised keysets matches it
fn resolve_keyset_id(short_id: &ShortKeysetId, mint_keysets: &[KeySetInfo]) -> Result<Id> {
    let mut matching = mint_keysets
        .iter()
        .filter(|keyset| ShortKeysetId::from(keyset.id) == *short_id);
    let keyset = matching
        .next()
        .ok_or_else(|| Error::UnknownKeysetId(short_id.clone()))?;
    ensure_cdk!(
        matching.next().is_none(),
        Error::AmbiguousKeysetId(short_id.clone())
    );
    Ok(keyset.id)
}

fn proofs_of(tokens: &[TokenV4Token], mint_keysets: &[KeySetInfo]) -> Result<Proofs> {
    let mut proofs = Proofs::with_capacity(tokens.iter().map(|t| t.proofs.len()).sum());
    for token in tokens {
        let keyset_id = resolve_keyset_id(&token.keyset_id, mint_keysets)?;
        proofs.extend(token.proofs.iter().map(|p| p.into_proof(&keyset_id)));
    }
    Ok(proofs)
}

/// Total value - errors if a secret is carried more than once, whatever amounts
/// the duplicates claim
fn value_of(tokens: &[TokenV4Token]) -> Result<Amount> {
    let mut secrets = HashSet::new();
    let mut total = Amount::ZERO;
    for proof in tokens.iter().flat_map(|token| &token.proofs) {
        ensure_cdk!(
            secrets.insert(proof.secret.as_bytes()),
            Error::DuplicateProofs
        );
        total = total
            .checked_add(proof.amount)
            .ok_or(cashu::amount::Error::AmountOverflow)?;
    }
    Ok(total)
}

/// Groups proofs by keyset id, keysets in id order and proofs in secret order, so
/// that the encoding of a set of proofs does not depend on how it was assembled
fn group_proofs(proofs: Proofs) -> Vec<TokenV4Token> {
    proofs
        .into_iter()
        .fold(BTreeMap::<Id, Proofs>::new(), |mut acc, proof| {
            acc.entry(proof.keyset_id).or_default().push(proof);
            acc
        })
        .into_iter()
        .map(|(id, mut proofs)| {
            proofs.sort_unstable_by(|a, b| a.secret.cmp(&b.secret));
            TokenV4Token::new(id, proofs)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        NETWORK_MAINNET, NETWORK_REGTEST, NETWORK_SIGNET, NETWORK_TESTNET, NETWORK_TESTNET4,
    };
    use crate::ecash::Proof;

    const MINT_ID: &str = "02b463e1f803480e0964a1f65b508b77e2e5d1d3054e94ba1d353b9db76e453da5";
    /// Legacy token in the canonical cbor field order, with no memo and no `d`
    const V4_MINIMAL: &str = "bitcrBo2FtdWh0dHA6Ly9sb2NhbGhvc3Q6MzMzOGF1Y3NhdGF0gaJhaUgArSaMTR9YJmFwgaNhYQFhc3hAOWE2ZGJiODQ3YmQyMzJiYTc2ZGIwZGYxOTcyMTZiMjlkM2I4Y2MxNDU1M2NkMjc4MjdmYzFjYzk0MmZlZGI0ZWFjWCEDhhhUP_trhpXfStS6vN6So0qWvc2X3O4NfM-Y1HISZ5I=";
    /// Legacy token whose proofs omit `d`, with the memo written after the proofs
    const V4_NO_DLEQ: &str = "bitcrBpGFtdWh0dHA6Ly9sb2NhbGhvc3Q6MzMzOGF1Y3NhdGF0gaJhaUgArSaMTR9YJmFwgaNhYQFhc3hAOWE2ZGJiODQ3YmQyMzJiYTc2ZGIwZGYxOTcyMTZiMjlkM2I4Y2MxNDU1M2NkMjc4MjdmYzFjYzk0MmZlZGI0ZWFjWCEDhhhUP_trhpXfStS6vN6So0qWvc2X3O4NfM-Y1HISZ5JhZGlUaGFuayB5b3U";
    /// Legacy token whose proofs carry a populated `d`
    const V4_WITH_DLEQ: &str = "bitcrBpGFteC9odHRwczovL21pbnQud2lsZGNhdDAuY2xvd2Rlci1kZXYubWluaWJpbGwudGVjaGF1Y3NhdGFkY3NhdGF0gaJhaUgBRjxG54nx4WFwh6RhYQRhc3hAYjdkNTUwZjljZjYyYzk0NmM3YzdjNDVkNjc4ZmUxMzc3OTdkNWRkMDMwZjQxMWY1NDg2OTU4MjE1MjRkYWY0M2FjWCECy1iLQBVPbznqF_cuQ_hj7sVg39ZGE-4aFvTnkPPIyMhhZKNhZVggmwzCLAxNKeNsJw_ZM-n1nfyE1bQSpXB9rvE8sLjGK_xhc1ggmYxKXPHd1N8yIRYn7KCa6Jl28EMVsQF-QwJHxXHe34thclggXUw1Z18UJiVzDdi0soIgil4JGA6iBRBHDeK_Qy1tYFSkYWEYgGFzeEAxMmJiNjg2YTAwNTg5N2MyYmRkZTBkODQxMzE0Mjg2NmIzMzkwZWI5Zjk5NzYxYmQ2NzVhYzI3YmY3NTFiZWI3YWNYIQP_o6JSVmMSfJT5I6fDZH7iZUXsLxmthXXtlSA8oVI3gGFko2FlWCCPbRLKnGTc1_WD_a91Fg-rr8P8cYk2gTZ8Qk19HJYWwWFzWCBLUfhonMlxUp4VVi-nxr-vDPag-o2ZZx0c-W7d3OSlHGFyWCCWjjuYWMj05LXrMebjwod7LWDYEVbskxEvAghk_GvCxqRhYQhhc3hAOTY0ZGYwOWQwOGQ5ZDRjY2E0YmVmYmM4ZWUzYjA4YWM5NTQ1ZjcxOTAwYWNkYzIzZWIzZDExZjUwNzIxZTFlN2FjWCEDRsrtPoGEFuuwRPhNSDKMGCZSuhPNQjh6bzuq_LTOrKdhZKNhZVggtp564XxyxpkRS_TirpegGZHyWXm4v4OAUTmzLO9ktnZhc1gg4fA_a2epcYSprnk4WAIyFNwkvuJCRsRvGEKpDHvw8RNhclggcLgcKktVlCP0ILVz53sP6tWK5Zn2p_0maxSMlD0-3jakYWEYQGFzeEA4NmJlNDBlNzRlOGVjYjc3YzA4NTdmYWEzMWY4MGQwZTQ1YzNlZjEzNzNjMTBjZWI1NWQzMzFhY2FhOWE5NjA0YWNYIQPDD9NSpAzTe8Aii_xweeyw2oa9nA5zz2CR4QdlYcr8JmFko2FlWCCuy6WOHlp4S-UHUIkQjaa4sNw8zkLBCSv-DMl8X6ASTWFzWCBvrg90GrXfBzLnWg4E0T8CA0jA5hVuHAVrLKZ9kLteq2FyWCCIP-6v7_r1nO1WblfP-Mkf6D_hL7ELH7HyzkURBxEhCKRhYQFhc3hAZmI3YjEwMzFiMjRkNDBjODBjZTA1MjNkNGM0YzQxYTQ3MWY4OGI3ZWIzYTk1OWVlZDA4ZjQ0YWVjYjdhNThjNGFjWCEDcX4YKFg8E2RWOMbJzWfm-f-0mlg89Cuy0ZFDYSB5-a1hZKNhZVggQJ2jupPgw1S-NMqPdCTq8OkhjOsolRfBN5unMlWlQTZhc1ggAssEObaR2OZEwXj01Q7GMQ4xfaENrggLxU48minMRtVhclgg2SjONoi2MlTIF9zUhXAbudms-8CLSpI8cVwl_f23dRCkYWEZAgBhc3hAODY1NjNkN2RjNzYzYjZkN2MxYjQ4MWJlMWIwMzY5OTRhNjFlMGZmNDU2MWZmZGJlMzQ1ZDc4NjM5ZTJmM2M2OGFjWCEDmaQqK0Ng1ou8FFpzezyjYuJ3VfvAqh_fURHANFya7yxhZKNhZVggCP8wUozvBK7B89fpGmkLC4dXGJbaf919pnND5zXAo1Fhc1ggAMn3Zb1qlMCUQhpsiSnBmFQDBUa8RCEuGxWamPSTfKlhclggjzGTRZcZfx6V7b3Qds7p-q-yipRB2x2DVG2Y2rGmu1qkYWEZAQBhc3hANDY3MDM3OTgxNGMzM2Y3OTRmNTdhOTdjNTQ5ZTFmZDE1YTQyYTE4NWVjZTM2MGFmN2ZkYjliMTYxOTMxN2YxZmFjWCED9QgfGg6NNmI27aVQTGIGOD_AOvCv6ULNk8Gls3gb-YthZKNhZVggXGVixAC9eS6SITqMI2eM6yDdFbXR6idgIUXBkZA9qxthc1ggrb-pKZrAjxBl-BI-IwEvzvChGoSdFFyKMNhCGkB8h0JhclggKyY-ukPdK5o3RcsOGcM44L3KnG_lG4z2CupyqkvWqLk=";
    /// Legacy token whose proofs carry an explicit `d: null`
    const V4_NULL_DLEQ: &str = "bitcrBo2FtdWh0dHA6Ly9sb2NhbGhvc3Q6NDM0M2F1ZWNyc2F0YXSBomFpSABp3j5af6uYYXCHpGFhGEBhc3hAODcyYmIxNzY0ODA3NDY2YWUxMDY2MGQxMjA5ODUxYzQ2MGJmZjJmNDZiY2YyZmJmM2QzY2NjY2QyYzllMzNiMGFjWCECgISwm2AJEFh3vxZKCNjnxx3pZ8BBav7a5AXLtMVQVjRhZPakYWEYgGFzeEBhY2QzYzI5YjlhZjEwYmM4MTdiOWUxNGFhMjllZjIxODkzYmZjZWMwMzFmYWQyM2IxOWExMDhjMzFhZmQyODMyYWNYIQIMmOnUpdbYTBtRceuCXy_qajysL6sG9CsvtRSBukjWO2Fk9qRhYRkCAGFzeEA4ZmU1NDNmOTMxYjA4MzhhOTA3NmMyMjljNzg1OWU3MTc0MTUzMGVmMGFiZWMyMzlkOWE0ZWNjOGEyMGNlYzRmYWNYIQPqj23wVNNNx42KP28By2a5i6N5TMkVU8lixcZ3aeiA7WFk9qRhYQRhc3hAMzk4YjYzMmU4MTZmNzQ4Njc1N2E3NTk5Mzc2YjlhYmFkMGFmNGQwMTVkYTQ0Mjk5Zjg2OGYxNWM4ODdmNDNjYmFjWCEDo8X2Y4JoRJ1hGSXDSVgQH-YXpFw_NYXtPIUv5xJcX-9hZPakYWEIYXN4QGJjNjM4NTYxN2Q2NjJkN2Q5NWIxNDBlMTU4Y2MzMTYwZjAzMmQxMWJiZGEzZWY3MDRhYzcyOTliM2EzYjQyOThhY1ghA_UAeY1dWx5QHqsvepcUK68xfHZJIbuRCaM45uN4t9vsYWT2pGFhGQEAYXN4QDFlNGQ1ZGI1MTc2MzU2YWEwZTI2MzJmZDlkYTUxMjYzYmY1M2EyMjFkNmNhZmE5Y2U4YTExMjg4MGNhMWQwZmZhY1ghAm3brXrx4F8HY8-YeC-msEuI9vfSzBKayKzab58A6xYwYWT2pGFhAWFzeEAwNzcyNTMyYTJkMjZkNDcyOTZjNzQ3NzMxN2NhZjQzOTdjZjA4MmM0ZjkwMzE4YWJjMDljZGRmZTEyMzFiYThlYWNYIQPeNBo_DX-qSXr52rqbwhGKWx9VNpaddKwORBP9-43JzmFk9g==";
    const MAINNET_V5: &str = "bitcrmCpGFrWCECtGPh-ANIDglkofZbUIt34uXR0wVOlLodNTudt25FPaVhbXVodHRwOi8vbG9jYWxob3N0OjQzNDNhdWVjcnNhdGF0gaJhaUgA_9SLj17PgGFwgaNhYQFhc3hAYWNjMTI0MzVlN2I4NDg0YzNjZjE4NTAxNDkyMThhZjkwZjcxNmE1MmJmNGE1ZWQzNDdlNDhlY2MxM2Y3NzM4OGFjWCECRFODGd5IXVW-07KaZCvuWHk3WrnnpiDhHki6SCQh888=";
    const REGTEST_EMPTY_V5: &str =
        "bitcrrCo2FrWCECtGPh-ANIDglkofZbUIt34uXR0wVOlLodNTudt25FPaVhdWNzYXRhdIA=";

    fn mint_key() -> bitcoin::secp256k1::PublicKey {
        bitcoin::secp256k1::PublicKey::from_str(MINT_ID).unwrap()
    }

    fn v4(token_str: &str) -> BitcrTokenV4 {
        let Token::BitcrV4(token) = Token::from_str(token_str).unwrap() else {
            panic!("expected a V4 token");
        };
        token
    }

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

    fn proof(keyset_id: Id, amount: u64, secret: &str) -> Proof {
        Proof {
            amount: Amount::from(amount),
            keyset_id,
            secret: cashu::secret::Secret::new(secret),
            c: cashu::PublicKey::from_hex(MINT_ID).unwrap(),
            witness: None,
            dleq: None,
            p2pk_e: None,
        }
    }

    #[test]
    fn test_token_str_round_trip() {
        let token = Token::from_str(V4_NO_DLEQ).unwrap();
        let inner = v4(V4_NO_DLEQ);
        assert_eq!(inner.token.len(), 1);
        assert_eq!(inner.token[0].keyset_id.to_string(), "00ad268c4d1f5826");
        assert_eq!(inner.mint_url.to_string(), "http://localhost:3338");
        assert_eq!(
            inner.token[0].keyset_id,
            ShortKeysetId::from_str("00ad268c4d1f5826").unwrap()
        );
        assert_eq!(inner.unit, cashu::CurrencyUnit::Sat);
        assert_eq!(Token::from_str(&token.to_string()).unwrap(), token);
    }

    #[test]
    fn incorrect_tokens() {
        let incorrect_prefix = "casshuAeyJ0b2tlbiI6W3sibWludCI6Imh0dHBzOi8vODMzMy5zcGFjZTozMzM4IiwicHJvb2ZzIjpbeyJhbW91bnQiOjIsImlkIjoiMDA5YTFmMjkzMjUzZTQxZSIsInNlY3JldCI6IjQwNzkxNWJjMjEyYmU2MWE3N2UzZTZkMmFlYjRjNzI3OTgwYmRhNTFjZDA2YTZhZmMyOWUyODYxNzY4YTc4MzciLCJDIjoiMDJiYzkwOTc5OTdkODFhZmIyY2M3MzQ2YjVlNDM0NWE5MzQ2YmQyYTUwNmViNzk1ODU5OGE3MmYwY2Y4NTE2M2VhIn0seyJhbW91bnQiOjgsImlkIjoiMDA5YTFmMjkzMjUzZTQxZSIsInNlY3JldCI6ImZlMTUxMDkzMTRlNjFkNzc1NmIwZjhlZTBmMjNhNjI0YWNhYTNmNGUwNDJmNjE0MzNjNzI4YzcwNTdiOTMxYmUiLCJDIjoiMDI5ZThlNTA1MGI4OTBhN2Q2YzA5NjhkYjE2YmMxZDVkNWZhMDQwZWExZGUyODRmNmVjNjlkNjEyOTlmNjcxMDU5In1dfV0sInVuaXQiOiJzYXQiLCJtZW1vIjoiVGhhbmsgeW91LiJ9";
        assert!(Token::from_str(incorrect_prefix).is_err());

        let no_prefix = "eyJ0b2tlbiI6W3sibWludCI6Imh0dHBzOi8vODMzMy5zcGFjZTozMzM4IiwicHJvb2ZzIjpbeyJhbW91bnQiOjIsImlkIjoiMDA5YTFmMjkzMjUzZTQxZSIsInNlY3JldCI6IjQwNzkxNWJjMjEyYmU2MWE3N2UzZTZkMmFlYjRjNzI3OTgwYmRhNTFjZDA2YTZhZmMyOWUyODYxNzY4YTc4MzciLCJDIjoiMDJiYzkwOTc5OTdkODFhZmIyY2M3MzQ2YjVlNDM0NWE5MzQ2YmQyYTUwNmViNzk1ODU5OGE3MmYwY2Y4NTE2M2VhIn0seyJhbW91bnQiOjgsImlkIjoiMDA5YTFmMjkzMjUzZTQxZSIsInNlY3JldCI6ImZlMTUxMDkzMTRlNjFkNzc1NmIwZjhlZTBmMjNhNjI0YWNhYTNmNGUwNDJmNjE0MzNjNzI4YzcwNTdiOTMxYmUiLCJDIjoiMDI5ZThlNTA1MGI4OTBhN2Q2YzA5NjhkYjE2YmMxZDVkNWZhMDQwZWExZGUyODRmNmVjNjlkNjEyOTlmNjcxMDU5In1dfV0sInVuaXQiOiJzYXQiLCJtZW1vIjoiVGhhbmsgeW91LiJ9";
        assert!(Token::from_str(no_prefix).is_err());

        let correct_token = "bitcrBo2F0gqJhaUgA_9SLj17PgGFwgaNhYQFhc3hAYWNjMTI0MzVlN2I4NDg0YzNjZjE4NTAxNDkyMThhZjkwZjcxNmE1MmJmNGE1ZWQzNDdlNDhlY2MxM2Y3NzM4OGFjWCECRFODGd5IXVW-07KaZCvuWHk3WrnnpiDhHki6SCQh88-iYWlIAK0mjE0fWCZhcIKjYWECYXN4QDEzMjNkM2Q0NzA3YTU4YWQyZTIzYWRhNGU5ZjFmNDlmNWE1YjRhYzdiNzA4ZWIwZDYxZjczOGY0ODMwN2U4ZWVhY1ghAjRWqhENhLSsdHrr2Cw7AFrKUL9Ffr1XN6RBT6w659lNo2FhAWFzeEA1NmJjYmNiYjdjYzY0MDZiM2ZhNWQ1N2QyMTc0ZjRlZmY4YjQ0MDJiMTc2OTI2ZDNhNTdkM2MzZGNiYjU5ZDU3YWNYIQJzEpxXGeWZN5qXSmJjY8MzxWyvwObQGr5G1YCCgHicY2FtdWh0dHA6Ly9sb2NhbGhvc3Q6MzMzOGF1Y3NhdA==";
        assert!(Token::from_str(correct_token).is_ok());
    }

    #[test]
    fn test_token_value() {
        let token = v4(V4_WITH_DLEQ);
        assert_eq!(token.value().unwrap(), cashu::Amount::from(973));
        assert_eq!(token.unit.to_string(), "sat");
        assert!(token.token[0].proofs.iter().all(|p| p.dleq.is_some()));
    }

    #[test]
    fn test_serde() {
        let token = Token::from_str(V4_NULL_DLEQ).unwrap();
        let token_json = serde_json::to_string(&token).unwrap();
        let deserialized_token: Token = serde_json::from_str(&token_json).unwrap();
        assert_eq!(token, deserialized_token);
    }

    /// A token already written in the canonical cbor field order survives parse and
    /// re-encode byte for byte. Tokens written in another field order are rewritten
    /// into the canonical one, so for those only the second pass is byte stable.
    #[test]
    fn test_token_v4_encoding_is_byte_stable() {
        assert_eq!(Token::from_str(V4_MINIMAL).unwrap().to_string(), V4_MINIMAL);

        for token_str in [V4_MINIMAL, V4_NO_DLEQ, V4_WITH_DLEQ, V4_NULL_DLEQ] {
            let token = Token::from_str(token_str).unwrap();
            let once = token.to_string();
            assert_eq!(Token::from_str(&once).unwrap(), token, "{token_str}");
            assert_eq!(
                Token::from_str(&once).unwrap().to_string(),
                once,
                "{token_str}"
            );

            let raw = token.to_raw_bytes().unwrap();
            assert_eq!(&raw[..RAW_PREFIX.len() + 1], b"brawB");
            assert_eq!(Token::try_from(&raw).unwrap(), token, "{token_str}");
        }
    }

    /// A token carrying `d: null` is normalised to the shorter form: the value is
    /// unchanged and the normalised form is itself stable
    #[test]
    fn test_token_v4_normalises_explicit_null_dleq() {
        let token = Token::from_str(V4_NULL_DLEQ).unwrap();
        let normalised = token.to_string();
        assert!(normalised.len() < V4_NULL_DLEQ.len());
        assert_eq!(
            Token::from_str(&normalised).unwrap().value().unwrap(),
            token.value().unwrap()
        );
        assert_eq!(
            Token::from_str(&normalised).unwrap().to_string(),
            normalised
        );
    }

    /// Tokens share the `bitcr<network>` namespace with node and bill ids, so a
    /// token version char must never be mistaken for the start of an id payload
    #[test]
    fn test_ids_are_not_tokens() {
        for network in [bitcoin::Network::Bitcoin, bitcoin::Network::Regtest] {
            let node_id = crate::core::NodeId::new(mint_key(), network).to_string();
            let bill_id = crate::core::BillId::new(mint_key(), network).to_string();
            assert!(Token::from_str(&node_id).is_err(), "{node_id}");
            assert!(Token::from_str(&bill_id).is_err(), "{bill_id}");
        }
    }

    #[test]
    fn test_token_v5_spec_vectors() {
        let mainnet = v5(MAINNET_V5);
        assert_eq!(mainnet.mint_id.network(), bitcoin::Network::Bitcoin);
        assert_eq!(mainnet.mint_id.pub_key(), mint_key());
        assert_eq!(
            mainnet.mint_url().unwrap().to_string(),
            "http://localhost:4343"
        );
        assert_eq!(mainnet.unit, cashu::CurrencyUnit::Custom("crsat".into()));
        assert_eq!(mainnet.memo, None);
        assert_eq!(mainnet.token.len(), 1);
        assert_eq!(mainnet.token[0].keyset_id.to_string(), "00ffd48b8f5ecf80");
        assert_eq!(mainnet.value().unwrap(), cashu::Amount::from(1));
        assert_eq!(mainnet.to_string(), MAINNET_V5);

        let regtest = v5(REGTEST_EMPTY_V5);
        assert_eq!(regtest.mint_id.network(), bitcoin::Network::Regtest);
        assert_eq!(regtest.mint_id.pub_key(), mint_key());
        assert_eq!(regtest.mint_url, None);
        assert_eq!(regtest.unit, cashu::CurrencyUnit::Sat);
        assert!(regtest.token.is_empty());
        assert_eq!(regtest.to_string(), REGTEST_EMPTY_V5);
    }

    #[test]
    fn test_token_v5_round_trip_every_network() {
        let mainnet = v5(MAINNET_V5);

        for network in [
            bitcoin::Network::Bitcoin,
            bitcoin::Network::Testnet,
            bitcoin::Network::Testnet4,
            bitcoin::Network::Signet,
            bitcoin::Network::Regtest,
        ] {
            let token = BitcrTokenV5 {
                mint_id: NodeId::new(mint_key(), network),
                ..mainnet.clone()
            };
            let expected = format!("{ID_PREFIX}{}{VERSION_V5}", network_char(&network));

            let encoded = token.to_string();
            assert!(encoded.starts_with(&expected), "{encoded}");
            assert_eq!(v5(&encoded), token);

            let raw = token.to_raw_bytes().unwrap();
            assert_eq!(
                &raw[..RAW_PREFIX.len() + 2],
                format!("{RAW_PREFIX}{}{VERSION_V5}", network_char(&network)).as_bytes()
            );
            assert_eq!(Token::try_from(&raw).unwrap(), Token::from(token));
        }
    }

    /// The network cannot contradict the mint identity: both come from the one
    /// [`NodeId`], and the prefix is what carries it
    #[test]
    fn test_token_v5_network_comes_from_the_mint_id() {
        let token = BitcrTokenV5::new(
            NodeId::new(mint_key(), bitcoin::Network::Signet),
            cashu::CurrencyUnit::Sat,
            vec![],
        );
        assert!(token.to_string().starts_with("bitcrsC"));
        assert_eq!(
            v5(&token.to_string()).mint_id,
            NodeId::new(mint_key(), bitcoin::Network::Signet)
        );
    }

    /// V4 keeps its version char where V5 keeps a network char, so the two
    /// alphabets must never overlap
    #[test]
    fn version_and_network_chars_are_disjoint() {
        let networks = [
            NETWORK_MAINNET,
            NETWORK_TESTNET,
            NETWORK_TESTNET4,
            NETWORK_SIGNET,
            NETWORK_REGTEST,
        ];
        for version in [VERSION_V4, VERSION_V5] {
            assert!(!networks.contains(&version), "{version} is a network char");
        }
        for network in networks {
            assert_eq!(
                Header::parse(&[network as u8, VERSION_V5 as u8])
                    .unwrap()
                    .len(),
                2
            );
        }
        assert_eq!(Header::parse(&[VERSION_V4 as u8]).unwrap(), Header::V4);
    }

    #[test]
    fn test_header_rejects_unknown_network_and_version() {
        assert!(matches!(
            Header::parse(b"xC"),
            Err(Error::UnknownNetwork('x'))
        ));
        assert!(matches!(
            Header::parse(b"mA"),
            Err(Error::UnknownVersion('A'))
        ));
        assert!(matches!(
            Header::parse(b"mD"),
            Err(Error::UnknownVersion('D'))
        ));
        assert!(matches!(Header::parse(b"m"), Err(Error::UnsupportedToken)));
        assert!(matches!(Header::parse(b""), Err(Error::UnsupportedToken)));
    }

    #[test]
    fn test_token_v5_rejects_unknown_network_and_version() {
        for token_str in [
            MAINNET_V5.replace("bitcrmC", "bitcrmA"),
            MAINNET_V5.replace("bitcrmC", "bitcrmD"),
            MAINNET_V5.replace("bitcrmC", "bitcrxC"),
            MAINNET_V5.replace("bitcrmC", "bitcr"),
        ] {
            assert!(Token::from_str(&token_str).is_err(), "{token_str}");
        }
    }

    /// A valid prefix with a corrupt body reports what actually went wrong instead
    /// of collapsing into `UnsupportedToken`
    #[test]
    fn test_corrupt_body_reports_the_real_error() {
        assert!(matches!(
            Token::from_str("bitcrmCZm9vYmFy"),
            Err(Error::CborDecode(_))
        ));
        assert!(matches!(
            Token::from_str("bitcrmC!!!!"),
            Err(Error::Base64(_))
        ));
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
        assert_eq!(token.mint_id.pub_key(), mint_key());
    }

    #[test]
    fn test_token_v5_padding_is_optional() {
        let padded = MAINNET_V5;
        let unpadded = padded.trim_end_matches('=');
        assert_ne!(padded, unpadded);
        assert_eq!(v5(padded), v5(unpadded));
    }

    /// A short keyset id names a keyset only when exactly one advertised keyset
    /// matches it: zero is as unusable as two
    #[test]
    fn test_keyset_id_must_match_exactly_one_mint_keyset() {
        let first = Id::from_str(&format!("01aabbccddeeff00{}", "11".repeat(25))).unwrap();
        let second = Id::from_str(&format!("01aabbccddeeff00{}", "22".repeat(25))).unwrap();
        assert_eq!(ShortKeysetId::from(first), ShortKeysetId::from(second));

        let token = BitcrTokenV5 {
            token: vec![TokenV4Token {
                keyset_id: ShortKeysetId::from(first),
                proofs: v5(MAINNET_V5).token[0].proofs.clone(),
            }],
            ..v5(MAINNET_V5)
        };

        assert!(token.proofs(&[keyset_info(first)]).is_ok());
        assert!(matches!(
            token.proofs(&[keyset_info(first), keyset_info(second)]),
            Err(Error::AmbiguousKeysetId(_))
        ));
        assert!(matches!(token.proofs(&[]), Err(Error::UnknownKeysetId(_))));
    }

    /// The same rule holds for the legacy format and for v1 keyset ids, which
    /// cashu widens without ever consulting the mint's keysets
    #[test]
    fn test_v4_keyset_id_must_also_be_advertised() {
        let token = v4(V4_NO_DLEQ);
        let known = Id::from_str("00ad268c4d1f5826").unwrap();
        let other = Id::from_str("00ffd48b8f5ecf80").unwrap();

        assert_eq!(token.proofs(&[keyset_info(known)]).unwrap().len(), 1);
        assert!(matches!(
            token.proofs(&[keyset_info(other)]),
            Err(Error::UnknownKeysetId(_))
        ));
        assert!(matches!(token.proofs(&[]), Err(Error::UnknownKeysetId(_))));
    }

    /// Reading a token offline sees what resolving it sees: same proofs, same
    /// order, same `y`
    #[test]
    fn test_groups_read_the_same_proofs_offline() {
        let first = Id::from_str("00ad268c4d1f5826").unwrap();
        let second = Id::from_str("00ffd48b8f5ecf80").unwrap();
        let token = Token::from(BitcrTokenV5::new(
            NodeId::new(mint_key(), bitcoin::Network::Bitcoin),
            cashu::CurrencyUnit::Sat,
            vec![
                proof(second, 2, "bbb"),
                proof(first, 1, "aaa"),
                proof(first, 8, "ccc"),
            ],
        ));

        let resolved = token
            .proofs(&[keyset_info(first), keyset_info(second)])
            .unwrap();
        let offline: Vec<_> = token
            .groups()
            .iter()
            .flat_map(|group| &group.proofs)
            .collect();

        assert_eq!(offline.len(), 3);
        assert_eq!(offline.len(), resolved.len());
        for (carried, proof) in offline.iter().zip(resolved.iter()) {
            assert_eq!(carried.secret, proof.secret);
            assert_eq!(carried.y().unwrap(), proof.y().unwrap());
        }
    }

    /// Reusing a secret is a duplicate no matter what amounts the copies claim
    #[test]
    fn test_value_rejects_reused_secret() {
        let id = Id::from_str("00ad268c4d1f5826").unwrap();
        let mint_id = NodeId::new(mint_key(), bitcoin::Network::Regtest);
        let unit = cashu::CurrencyUnit::Sat;

        for amounts in [(1, 8), (1, 1)] {
            let token = BitcrTokenV5::new(
                mint_id.clone(),
                unit.clone(),
                vec![
                    proof(id, amounts.0, "reused"),
                    proof(id, amounts.1, "reused"),
                ],
            );
            assert!(matches!(token.value(), Err(Error::DuplicateProofs)));
        }

        let token = BitcrTokenV5::new(
            mint_id,
            unit,
            vec![proof(id, 1, "one"), proof(id, 8, "two")],
        );
        assert_eq!(token.value().unwrap(), Amount::from(9));
    }

    /// The encoding of a set of proofs must not depend on the order it was
    /// assembled in, nor on hash map iteration order
    #[test]
    fn test_encoding_is_canonical() {
        let first = Id::from_str("00ad268c4d1f5826").unwrap();
        let second = Id::from_str("00ffd48b8f5ecf80").unwrap();
        let mint_id = NodeId::new(mint_key(), bitcoin::Network::Regtest);
        let proofs = [
            proof(first, 1, "a"),
            proof(second, 2, "b"),
            proof(first, 4, "c"),
            proof(second, 8, "d"),
        ];

        let canonical =
            BitcrTokenV5::new(mint_id.clone(), cashu::CurrencyUnit::Sat, proofs.to_vec())
                .to_string();
        for rotation in 1..proofs.len() {
            let mut shuffled = proofs.to_vec();
            shuffled.rotate_left(rotation);
            shuffled.reverse();
            let encoded =
                BitcrTokenV5::new(mint_id.clone(), cashu::CurrencyUnit::Sat, shuffled).to_string();
            assert_eq!(encoded, canonical, "rotation {rotation}");
        }
    }

    #[test]
    fn test_builders_are_not_transposable() {
        let token = BitcrTokenV5::new(
            NodeId::new(mint_key(), bitcoin::Network::Regtest),
            cashu::CurrencyUnit::Sat,
            vec![],
        )
        .with_mint_url("http://mint")
        .with_memo("thanks");
        assert_eq!(token.mint_url.as_deref(), Some("http://mint"));
        assert_eq!(token.memo.as_deref(), Some("thanks"));
        assert_eq!(v5(&token.to_string()), token);
    }

    /// A hint that does not parse is reported as absent, the raw field keeps it
    #[test]
    fn test_mint_url_hint_is_best_effort() {
        let token = BitcrTokenV5::new(
            NodeId::new(mint_key(), bitcoin::Network::Regtest),
            cashu::CurrencyUnit::Sat,
            vec![],
        )
        .with_mint_url("not a url");
        assert_eq!(token.mint_url(), None);
        assert_eq!(token.mint_url.as_deref(), Some("not a url"));
    }
}
