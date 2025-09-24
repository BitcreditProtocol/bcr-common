// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::core::NodeId;

// ----- end imports

#[repr(u8)]
#[derive(
    Debug,
    Copy,
    Clone,
    serde_repr::Serialize_repr,
    serde_repr::Deserialize_repr,
    PartialEq,
    Eq,
    ToSchema,
    strum::FromRepr,
)]
pub enum IdentityType {
    Ident = 0,
    Anon = 1,
}

impl TryFrom<u64> for IdentityType {
    type Error = std::fmt::Error;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        IdentityType::from_repr(value as u8).ok_or(Self::Error {})
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SeedPhrase {
    pub seed_phrase: bip39::Mnemonic,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Identity {
    pub node_id: NodeId,
    pub name: String,
    pub email: Option<String>,
    pub bitcoin_public_key: bitcoin::PublicKey,
    pub npub: String,
    pub postal_address: OptionalPostalAddress,
    pub date_of_birth: Option<NaiveDate>,
    pub country_of_birth: Option<String>,
    pub city_of_birth: Option<String>,
    pub identification_number: Option<String>,
    pub profile_picture_file: Option<File>,
    pub identity_document_file: Option<File>,
    pub nostr_relays: Vec<url::Url>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewIdentityPayload {
    pub t: u64,
    pub name: String,
    pub email: Option<String>,
    pub postal_address: OptionalPostalAddress,
    pub date_of_birth: Option<NaiveDate>,
    pub country_of_birth: Option<String>,
    pub city_of_birth: Option<String>,
    pub identification_number: Option<String>,
    pub profile_picture_file_upload_id: Option<String>,
    pub identity_document_file_upload_id: Option<String>,
}

#[derive(
    Debug, Default, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize, ToSchema,
)]
pub struct PostalAddress {
    pub country: String,
    pub city: String,
    pub zip: Option<String>,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize, ToSchema)]
pub struct OptionalPostalAddress {
    pub country: Option<String>,
    pub city: Option<String>,
    pub zip: Option<String>,
    pub address: Option<String>,
}

impl OptionalPostalAddress {
    pub fn is_none(&self) -> bool {
        self.country.is_none()
            && self.city.is_none()
            && self.zip.is_none()
            && self.address.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct File {
    pub name: String,
    pub hash: String,
    pub nostr_hash: String,
}
