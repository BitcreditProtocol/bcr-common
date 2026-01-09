use bitcoin::base64::engine::{GeneralPurpose, general_purpose};
use bitcoin::base64::{Engine as _, alphabet};
use cashu::{
    Amount, CurrencyUnit, KeySetInfo, MintUrl, Proof, Proofs,
    nut00::{Error, ProofV4, token::TokenV4Token},
    nuts::Id,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

pub type CashuTokenV4 = cashu::nut00::TokenV4;

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
#[serde(untagged)]
pub enum Token {
    BitcrV4(BitcrTokenV4),
    CashuV4(CashuTokenV4),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let token = match self {
            Self::BitcrV4(token) => token.to_string(),
            Self::CashuV4(token) => token.to_string(),
        };

        write!(f, "{token}")
    }
}

impl Token {
    /// Create new bitcrV4 [`Token`]
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

    /// Create new cashuV4 [`Token`]
    pub fn new_cashu(
        mint_url: MintUrl,
        proofs: Proofs,
        memo: Option<String>,
        unit: CurrencyUnit,
    ) -> Self {
        let proofs = proofs_to_tokenv4(proofs);

        Self::CashuV4(CashuTokenV4 {
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
            Self::CashuV4(token) => token.proofs(mint_keysets),
        }
    }

    /// Total value of [`Token`]
    pub fn value(&self) -> Result<Amount, Error> {
        match self {
            Self::BitcrV4(token) => token.value(),
            Self::CashuV4(token) => token.value(),
        }
    }

    /// [`Token`] memo
    pub fn memo(&self) -> &Option<String> {
        match self {
            Self::BitcrV4(token) => token.memo(),
            Self::CashuV4(token) => token.memo(),
        }
    }

    /// Unit
    pub fn unit(&self) -> Option<CurrencyUnit> {
        match self {
            Self::BitcrV4(token) => Some(token.unit().clone()),
            Self::CashuV4(token) => Some(token.unit().clone()),
        }
    }

    /// Mint url
    pub fn mint_url(&self) -> MintUrl {
        match self {
            Self::BitcrV4(token) => token.mint_url.clone(),
            Self::CashuV4(token) => token.mint_url.clone(),
        }
    }

    /// Serialize the token to raw binary
    pub fn to_raw_bytes(&self) -> Result<Vec<u8>, Error> {
        match self {
            Self::BitcrV4(_) => Err(Error::UnsupportedToken),
            Self::CashuV4(token) => token.to_raw_bytes(),
        }
    }
}

impl FromStr for Token {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match (CashuTokenV4::from_str(s), BitcrTokenV4::from_str(s)) {
            (Ok(token), Err(_)) => Ok(Token::CashuV4(token)),
            (Err(_), Ok(token)) => Ok(Token::BitcrV4(token)),
            _ => Err(Error::UnsupportedToken),
        }
    }
}

impl TryFrom<&Vec<u8>> for Token {
    type Error = Error;

    fn try_from(bytes: &Vec<u8>) -> Result<Self, Self::Error> {
        if let Ok(token) = CashuTokenV4::try_from(bytes) {
            return Ok(Token::CashuV4(token));
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
        let proofs: Vec<&ProofV4> = self.token.iter().flat_map(|t| &t.proofs).collect();
        let unique_count = proofs
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();

        // Check if there are any duplicate proofs
        if unique_count != proofs.len() {
            return Err(Error::DuplicateProofs);
        }

        Ok(Amount::try_sum(
            self.token
                .iter()
                .map(|t| Amount::try_sum(t.proofs.iter().map(|p| p.amount)))
                .collect::<Result<Vec<Amount>, _>>()?,
        )?)
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

        let decode_config = general_purpose::GeneralPurposeConfig::new()
            .with_decode_padding_mode(bitcoin::base64::engine::DecodePaddingMode::Indifferent);
        let decoded = GeneralPurpose::new(&alphabet::URL_SAFE, decode_config).decode(s)?;
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
    fn test_token_str_round_trip_1() {
        let token_str = "cashuBpGF0gaJhaUgArSaMTR9YJmFwgaNhYQFhc3hAOWE2ZGJiODQ3YmQyMzJiYTc2ZGIwZGYxOTcyMTZiMjlkM2I4Y2MxNDU1M2NkMjc4MjdmYzFjYzk0MmZlZGI0ZWFjWCEDhhhUP_trhpXfStS6vN6So0qWvc2X3O4NfM-Y1HISZ5JhZGlUaGFuayB5b3VhbXVodHRwOi8vbG9jYWxob3N0OjMzMzhhdWNzYXQ=";

        let token = Token::from_str(token_str).unwrap();
        assert!(matches!(token, Token::CashuV4(_)));
        let Token::CashuV4(inner) = token.clone() else {
            panic!("Expected CashuV4 token");
        };
        assert_eq!(inner.token.len(), 1);
        assert_eq!(inner.token[0].keyset_id.to_string(), "00ad268c4d1f5826");
        let _ = token.to_string().strip_prefix("cashuB").expect("prefix");
        assert_eq!(inner.mint_url.to_string(), "http://localhost:3338");
        assert_eq!(
            inner.token[0].keyset_id,
            cdk02::ShortKeysetId::from_str("00ad268c4d1f5826").unwrap()
        );
        assert_eq!(inner.unit.clone(), cashu::CurrencyUnit::Sat);

        let encoded = &inner.to_string();

        let token_data = CashuTokenV4::from_str(encoded).unwrap();
        assert_eq!(token_data, inner);
    }

    #[test]
    fn test_token_str_round_trip_2() {
        let token_str = "bitcrBpGFtdWh0dHA6Ly9sb2NhbGhvc3Q6MzMzOGF1ZWNyc2F0YXSBomFpSACtJoxNH1gmYXCBo2FhAWFzeEA5YTZkYmI4NDdiZDIzMmJhNzZkYjBkZjE5NzIxNmIyOWQzYjhjYzE0NTUzY2QyNzgyN2ZjMWNjOTQyZmVkYjRlYWNYIQOGGFQ_-2uGld9K1Lq83pKjSpa9zZfc7g18z5jUchJnkmFkaVRoYW5rIHlvdQ";

        let token = Token::from_str(token_str).unwrap();
        assert!(matches!(token, Token::BitcrV4(_)));
        let Token::BitcrV4(inner) = token.clone() else {
            panic!("Expected BitcrV4 token");
        };
        assert_eq!(inner.token.len(), 1);
        assert_eq!(inner.token[0].keyset_id.to_string(), "00ad268c4d1f5826");

        token.to_string().strip_prefix("bitcrB").unwrap();
        assert_eq!(inner.mint_url.to_string(), "http://localhost:3338");
        assert_eq!(
            inner.token[0].keyset_id,
            cdk02::ShortKeysetId::from_str("00ad268c4d1f5826").unwrap()
        );
        assert_eq!(
            inner.unit.clone(),
            cashu::CurrencyUnit::Custom(String::from("crsat"))
        );

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

        let correct_token = "cashuBo2F0gqJhaUgA_9SLj17PgGFwgaNhYQFhc3hAYWNjMTI0MzVlN2I4NDg0YzNjZjE4NTAxNDkyMThhZjkwZjcxNmE1MmJmNGE1ZWQzNDdlNDhlY2MxM2Y3NzM4OGFjWCECRFODGd5IXVW-07KaZCvuWHk3WrnnpiDhHki6SCQh88-iYWlIAK0mjE0fWCZhcIKjYWECYXN4QDEzMjNkM2Q0NzA3YTU4YWQyZTIzYWRhNGU5ZjFmNDlmNWE1YjRhYzdiNzA4ZWIwZDYxZjczOGY0ODMwN2U4ZWVhY1ghAjRWqhENhLSsdHrr2Cw7AFrKUL9Ffr1XN6RBT6w659lNo2FhAWFzeEA1NmJjYmNiYjdjYzY0MDZiM2ZhNWQ1N2QyMTc0ZjRlZmY4YjQ0MDJiMTc2OTI2ZDNhNTdkM2MzZGNiYjU5ZDU3YWNYIQJzEpxXGeWZN5qXSmJjY8MzxWyvwObQGr5G1YCCgHicY2FtdWh0dHA6Ly9sb2NhbGhvc3Q6MzMzOGF1Y3NhdA==";

        let correct_token = Token::from_str(correct_token);

        assert!(correct_token.is_ok());
    }

    #[test]
    fn test_token_value() {
        let token_str = "bitcrBo2FtdWh0dHA6Ly9sb2NhbGhvc3Q6NDM0M2F1ZWNyc2F0YXSBomFpSABp3j5af6uYYXCHpGFhGEBhc3hAODcyYmIxNzY0ODA3NDY2YWUxMDY2MGQxMjA5ODUxYzQ2MGJmZjJmNDZiY2YyZmJmM2QzY2NjY2QyYzllMzNiMGFjWCECgISwm2AJEFh3vxZKCNjnxx3pZ8BBav7a5AXLtMVQVjRhZPakYWEYgGFzeEBhY2QzYzI5YjlhZjEwYmM4MTdiOWUxNGFhMjllZjIxODkzYmZjZWMwMzFmYWQyM2IxOWExMDhjMzFhZmQyODMyYWNYIQIMmOnUpdbYTBtRceuCXy_qajysL6sG9CsvtRSBukjWO2Fk9qRhYRkCAGFzeEA4ZmU1NDNmOTMxYjA4MzhhOTA3NmMyMjljNzg1OWU3MTc0MTUzMGVmMGFiZWMyMzlkOWE0ZWNjOGEyMGNlYzRmYWNYIQPqj23wVNNNx42KP28By2a5i6N5TMkVU8lixcZ3aeiA7WFk9qRhYQRhc3hAMzk4YjYzMmU4MTZmNzQ4Njc1N2E3NTk5Mzc2YjlhYmFkMGFmNGQwMTVkYTQ0Mjk5Zjg2OGYxNWM4ODdmNDNjYmFjWCEDo8X2Y4JoRJ1hGSXDSVgQH-YXpFw_NYXtPIUv5xJcX-9hZPakYWEIYXN4QGJjNjM4NTYxN2Q2NjJkN2Q5NWIxNDBlMTU4Y2MzMTYwZjAzMmQxMWJiZGEzZWY3MDRhYzcyOTliM2EzYjQyOThhY1ghA_UAeY1dWx5QHqsvepcUK68xfHZJIbuRCaM45uN4t9vsYWT2pGFhGQEAYXN4QDFlNGQ1ZGI1MTc2MzU2YWEwZTI2MzJmZDlkYTUxMjYzYmY1M2EyMjFkNmNhZmE5Y2U4YTExMjg4MGNhMWQwZmZhY1ghAm3brXrx4F8HY8-YeC-msEuI9vfSzBKayKzab58A6xYwYWT2pGFhAWFzeEAwNzcyNTMyYTJkMjZkNDcyOTZjNzQ3NzMxN2NhZjQzOTdjZjA4MmM0ZjkwMzE4YWJjMDljZGRmZTEyMzFiYThlYWNYIQPeNBo_DX-qSXr52rqbwhGKWx9VNpaddKwORBP9-43JzmFk9g==";

        let token = Token::from_str(token_str).unwrap();
        token.to_string().strip_prefix("bitcrB").unwrap();
        if let Token::BitcrV4(token) = token {
            assert_eq!(token.value().unwrap(), cashu::Amount::from(973));
            assert_eq!(token.unit.to_string(), "crsat");
        }
    }

    #[test]
    fn test_serde() {
        let token_str = "bitcrBo2FtdWh0dHA6Ly9sb2NhbGhvc3Q6NDM0M2F1ZWNyc2F0YXSBomFpSABp3j5af6uYYXCHpGFhGEBhc3hAODcyYmIxNzY0ODA3NDY2YWUxMDY2MGQxMjA5ODUxYzQ2MGJmZjJmNDZiY2YyZmJmM2QzY2NjY2QyYzllMzNiMGFjWCECgISwm2AJEFh3vxZKCNjnxx3pZ8BBav7a5AXLtMVQVjRhZPakYWEYgGFzeEBhY2QzYzI5YjlhZjEwYmM4MTdiOWUxNGFhMjllZjIxODkzYmZjZWMwMzFmYWQyM2IxOWExMDhjMzFhZmQyODMyYWNYIQIMmOnUpdbYTBtRceuCXy_qajysL6sG9CsvtRSBukjWO2Fk9qRhYRkCAGFzeEA4ZmU1NDNmOTMxYjA4MzhhOTA3NmMyMjljNzg1OWU3MTc0MTUzMGVmMGFiZWMyMzlkOWE0ZWNjOGEyMGNlYzRmYWNYIQPqj23wVNNNx42KP28By2a5i6N5TMkVU8lixcZ3aeiA7WFk9qRhYQRhc3hAMzk4YjYzMmU4MTZmNzQ4Njc1N2E3NTk5Mzc2YjlhYmFkMGFmNGQwMTVkYTQ0Mjk5Zjg2OGYxNWM4ODdmNDNjYmFjWCEDo8X2Y4JoRJ1hGSXDSVgQH-YXpFw_NYXtPIUv5xJcX-9hZPakYWEIYXN4QGJjNjM4NTYxN2Q2NjJkN2Q5NWIxNDBlMTU4Y2MzMTYwZjAzMmQxMWJiZGEzZWY3MDRhYzcyOTliM2EzYjQyOThhY1ghA_UAeY1dWx5QHqsvepcUK68xfHZJIbuRCaM45uN4t9vsYWT2pGFhGQEAYXN4QDFlNGQ1ZGI1MTc2MzU2YWEwZTI2MzJmZDlkYTUxMjYzYmY1M2EyMjFkNmNhZmE5Y2U4YTExMjg4MGNhMWQwZmZhY1ghAm3brXrx4F8HY8-YeC-msEuI9vfSzBKayKzab58A6xYwYWT2pGFhAWFzeEAwNzcyNTMyYTJkMjZkNDcyOTZjNzQ3NzMxN2NhZjQzOTdjZjA4MmM0ZjkwMzE4YWJjMDljZGRmZTEyMzFiYThlYWNYIQPeNBo_DX-qSXr52rqbwhGKWx9VNpaddKwORBP9-43JzmFk9g==";

        let token = Token::from_str(token_str).unwrap();
        let token_json = serde_json::to_string(&token).unwrap();
        let deserialized_token: Token = serde_json::from_str(&token_json).unwrap();
        assert_eq!(token, deserialized_token);
    }
}
