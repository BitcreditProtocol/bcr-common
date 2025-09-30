// ----- standard library imports
use std::str::FromStr;
// ----- extra library imports
use borsh::io::{Error as BorshError, ErrorKind, Read, Write};
// ----- local imports

// ----- end imports

type Result<T> = core::result::Result<T, BorshError>;

pub fn serialize_cdk_pubkey<W: Write>(key: &cashu::PublicKey, writer: &mut W) -> Result<()> {
    let pubkey_str = key.to_string();
    borsh::BorshSerialize::serialize(&pubkey_str, writer)?;
    Ok(())
}
pub fn deserialize_cdk_pubkey<R: Read>(reader: &mut R) -> Result<cashu::PublicKey> {
    let pubkey_str: String = borsh::BorshDeserialize::deserialize_reader(reader)?;
    let pubkey = cashu::PublicKey::from_str(&pubkey_str)
        .map_err(|e| BorshError::new(ErrorKind::InvalidInput, e))?;
    Ok(pubkey)
}

pub fn serialize_vec_url<W: std::io::Write>(
    vec: &[url::Url],
    writer: &mut W,
) -> std::io::Result<()> {
    let url_strs: Vec<String> = vec.iter().map(|u| u.to_string()).collect();
    borsh::BorshSerialize::serialize(&url_strs, writer)?;
    Ok(())
}

pub fn deserialize_vec_url<R: std::io::Read>(reader: &mut R) -> std::io::Result<Vec<url::Url>> {
    let url_strs: Vec<String> = borsh::BorshDeserialize::deserialize_reader(reader)?;
    url_strs
        .into_iter()
        .map(|s| {
            url::Url::from_str(&s)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })
        .collect()
}

pub fn serialize_chrono_naivedate<W: Write>(
    date: &chrono::NaiveDate,
    writer: &mut W,
) -> Result<()> {
    let date_str = date.to_string();
    borsh::BorshSerialize::serialize(&date_str, writer)?;
    Ok(())
}

pub fn deserialize_chrono_naivedate<R: Read>(reader: &mut R) -> Result<chrono::NaiveDate> {
    let date_str: String = borsh::BorshDeserialize::deserialize_reader(reader)?;
    let date = chrono::NaiveDate::from_str(&date_str)
        .map_err(|e| BorshError::new(ErrorKind::InvalidInput, e))?;
    Ok(date)
}

pub fn serialize_btc_pubkey<W: Write>(key: &bitcoin::PublicKey, writer: &mut W) -> Result<()> {
    let pubkey_str = key.to_string();
    borsh::BorshSerialize::serialize(&pubkey_str, writer)?;
    Ok(())
}
pub fn deserialize_btc_pubkey<R: Read>(reader: &mut R) -> Result<bitcoin::PublicKey> {
    let pubkey_str: String = borsh::BorshDeserialize::deserialize_reader(reader)?;
    let pubkey = bitcoin::PublicKey::from_str(&pubkey_str)
        .map_err(|e| BorshError::new(ErrorKind::InvalidInput, e))?;
    Ok(pubkey)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_chrono_naivedate() {
        let date = chrono::NaiveDate::from_ymd_opt(2023, 10, 5).unwrap();
        let mut buf = Vec::new();
        serialize_chrono_naivedate(&date, &mut buf).unwrap();
        let deserialized_date = deserialize_chrono_naivedate(&mut buf.as_slice()).unwrap();
        assert_eq!(date, deserialized_date);
    }

    #[test]
    fn test_serialize_deserialize_btc_pubkey() {
        let pubkey_str = "02c0ded8f7b5e6c3f4e8b6a1e4f3c2d1e0f9e8d7c6b5a4e3f2d1c0b9a8e7f6d5c4";
        let pubkey = bitcoin::PublicKey::from_str(pubkey_str).unwrap();
        let mut buf = Vec::new();
        serialize_btc_pubkey(&pubkey, &mut buf).unwrap();
        let deserialized_pubkey = deserialize_btc_pubkey(&mut buf.as_slice()).unwrap();
        assert_eq!(pubkey, deserialized_pubkey);
    }
}
