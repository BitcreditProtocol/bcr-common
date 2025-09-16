// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use utoipa::ToSchema;
// ----- local modules
// ----- end imports

#[repr(u8)]
#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    serde_repr::Serialize_repr,
    serde_repr::Deserialize_repr,
    PartialEq,
    Eq,
    BorshSerialize,
    BorshDeserialize,
    ToSchema,
    strum::FromRepr,
)]
#[borsh(use_discriminant = true)]
pub enum ContactType {
    #[default]
    Person = 0,
    Company = 1,
    Anon = 2,
}

impl TryFrom<u64> for ContactType {
    type Error = std::fmt::Error;

    fn try_from(value: u64) -> std::result::Result<Self, Self::Error> {
        ContactType::from_repr(value as u8).ok_or(Self::Error {})
    }
}
