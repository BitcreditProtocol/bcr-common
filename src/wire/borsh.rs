// ----- standard library imports
use std::str::FromStr;
// ----- extra library imports
use borsh::io::{Error as BorshError, ErrorKind, Read, Write};
// ----- local imports

// ----- end imports

type Result<T> = core::result::Result<T, BorshError>;

pub fn serialize_as_str<T>(t: &T, writer: &mut impl Write) -> Result<()>
where
    T: std::fmt::Display,
{
    let stringified = t.to_string();
    borsh::BorshSerialize::serialize(&stringified, writer)?;
    Ok(())
}
pub fn deserialize_as_str<T>(reader: &mut impl Read) -> Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    let stringified: String = borsh::BorshDeserialize::deserialize_reader(reader)?;
    let t = T::from_str(&stringified).map_err(|e| BorshError::new(ErrorKind::InvalidInput, e))?;
    Ok(t)
}

pub fn serialize_vec_of_strs<T>(vec: &[T], writer: &mut impl Write) -> Result<()>
where
    T: std::fmt::Display,
{
    let strs: Vec<String> = vec.iter().map(|v| v.to_string()).collect();
    borsh::BorshSerialize::serialize(&strs, writer)?;
    Ok(())
}

pub fn deserialize_vec_of_strs<T>(reader: &mut impl Read) -> Result<Vec<T>>
where
    T: std::str::FromStr,
    <T as FromStr>::Err: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    let strs: Vec<String> = borsh::BorshDeserialize::deserialize_reader(reader)?;
    strs.into_iter()
        .map(|v| T::from_str(&v))
        .collect::<std::result::Result<Vec<T>, T::Err>>()
        .map_err(|e| BorshError::new(ErrorKind::InvalidData, e))
}
pub fn serialize_vec_of_jsons<T>(vec: &[T], writer: &mut impl Write) -> Result<()>
where
    T: serde::ser::Serialize,
{
    let stringified = serde_json::to_string(vec)
        .map_err(|e| BorshError::new(ErrorKind::InvalidInput, e))?;
    borsh::BorshSerialize::serialize(&stringified, writer)?;
    Ok(())
}

pub fn deserialize_vec_of_jsons<T>(reader: &mut impl Read) -> Result<Vec<T>>
where
    T: serde::de::DeserializeOwned,
{
    let stringified: String = borsh::BorshDeserialize::deserialize_reader(reader)?;
    let vec = serde_json::from_str(&stringified)
        .map_err(|e| BorshError::new(ErrorKind::InvalidData, e))?;
    Ok(vec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_tests;

    #[test]
    fn serialize_deserialize_chrono_naivedate() {
        let date = chrono::NaiveDate::from_ymd_opt(2023, 10, 5).unwrap();
        let mut buf = Vec::new();
        serialize_as_str(&date, &mut buf).unwrap();
        let deserialized_date = deserialize_as_str(&mut buf.as_slice()).unwrap();
        assert_eq!(date, deserialized_date);
    }

    #[test]
    fn serialize_deserialize_btc_pubkey() {
        let pubkey_str = "02c0ded8f7b5e6c3f4e8b6a1e4f3c2d1e0f9e8d7c6b5a4e3f2d1c0b9a8e7f6d5c4";
        let pubkey = bitcoin::PublicKey::from_str(pubkey_str).unwrap();
        let mut buf = Vec::new();
        serialize_as_str(&pubkey, &mut buf).unwrap();
        let deserialized_pubkey = deserialize_as_str(&mut buf.as_slice()).unwrap();
        assert_eq!(pubkey, deserialized_pubkey);
    }

    #[test]
    fn serialize_deserialize_chrono_tstamp() {
        let tstamp = chrono::Utc::now();
        let mut buf = Vec::new();
        serialize_as_str(&tstamp, &mut buf).unwrap();
        let deserialized_tstamp: chrono::DateTime<chrono::Utc> =
            deserialize_as_str(&mut buf.as_slice()).unwrap();
        assert_eq!(tstamp, deserialized_tstamp);
    }

    #[test]
    fn serialize_deserialize_vec_of_jsons_cdk_proofs() {
        let (_, keyset) = core_tests::generate_random_ecash_keyset();
        let amount = cashu::Amount::from_str("1000").unwrap();
        let proofs = core_tests::generate_random_ecash_proofs(&keyset, &amount.split());
        let mut buf = Vec::new();
        serialize_vec_of_jsons(&proofs, &mut buf).unwrap();
        let deserialized = deserialize_vec_of_jsons(&mut buf.as_slice()).unwrap();
        assert_eq!(proofs, deserialized);
    }

    #[test]
    fn serialize_deserialize_vec_of_strs_cdk_pubkeys() {
        let pks: Vec<_> = std::iter::repeat_with(|| {
            cashu::PublicKey::from(core_tests::generate_random_keypair().public_key())
        })
        .take(5)
        .collect();
        let mut buf = Vec::new();
        serialize_vec_of_strs(&pks, &mut buf).unwrap();
        let deserialized = deserialize_vec_of_strs(&mut buf.as_slice()).unwrap();
        assert_eq!(pks, deserialized);
    }

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Field {
        pub f1: String,
        pub f2: u8,
    }
    #[derive(borsh::BorshSerialize, borsh::BorshDeserialize, PartialEq, Debug)]
    struct Test {
        pub f1: String,
        #[borsh(
            serialize_with = "serialize_vec_of_jsons",
            deserialize_with = "deserialize_vec_of_jsons"
        )]
        pub f2: Vec<Field>,
        pub f3: Vec<u32>,
    }

    #[test]
    fn serialize_deserialize_struct() {
        let t = Test {
            f1: String::from("field 1"),
            f2: vec![
                Field {
                    f1: String::from("a"),
                    f2: 1,
                },
                Field {
                    f1: String::from("b"),
                    f2: 2,
                },
            ],
            f3: vec![10, 20, 30],
        };
        let mut buf = Vec::new();
        borsh::BorshSerialize::serialize(&t, &mut buf).unwrap();
        let deserialized_t = borsh::BorshDeserialize::deserialize_reader(&mut buf.as_slice()).unwrap();
        assert_eq!(t, deserialized_t);
    }
}
