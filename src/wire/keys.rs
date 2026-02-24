// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::{
    core,
    wire::borsh::{
        deserialize_from_str, deserialize_optionproofdleq, deserialize_optionwitness,
        serialize_as_str, serialize_optionproofdleq, serialize_optionwitness,
    },
};

// ----- end imports

///--------------------------- Generate keyset
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct EnableNewMintingOpRequest {
    pub kid: cashu::Id,
    pub condition: KeysetMintCondition,
    pub expire: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct KeysetMintCondition {
    pub amount: cashu::Amount,
    #[schema(value_type = String)]
    pub public_key: cashu::PublicKey,
}
///--------------------------- Pre-sign blinded message
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct SignRequest {
    pub kid: cashu::Id,
    pub msg: cashu::BlindedMessage,
}

///--------------------------- Deactivate keyset
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct DeactivateKeysetRequest {
    pub kid: cashu::Id,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct DeactivateKeysetResponse {
    pub kid: cashu::Id,
}

///--------------------------- Proof fingerprint validation
#[derive(
    Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize, ToSchema, PartialEq,
)]
pub struct ProofFingerprint {
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub keyset_id: cashu::Id,
    pub amount: u64,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub y: cashu::PublicKey, // Y = hash_to_curve(secret)
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub c: cashu::PublicKey, // unblinded signature
    #[borsh(
        serialize_with = "serialize_optionproofdleq",
        deserialize_with = "deserialize_optionproofdleq"
    )]
    pub dleq: Option<cashu::ProofDleq>,
    #[borsh(
        serialize_with = "serialize_optionwitness",
        deserialize_with = "deserialize_optionwitness"
    )]
    pub witness: Option<cashu::Witness>,
}

impl std::convert::From<ProofFingerprint> for core::signature::ProofFingerprint {
    fn from(fp: ProofFingerprint) -> Self {
        core::signature::ProofFingerprint {
            keyset_id: fp.keyset_id,
            amount: cashu::Amount::from(fp.amount),
            y: *fp.y,
            c: *fp.c,
        }
    }
}

impl std::convert::TryFrom<cashu::Proof> for ProofFingerprint {
    type Error = cashu::nut00::Error;
    fn try_from(proof: cashu::Proof) -> std::result::Result<Self, Self::Error> {
        let y = proof.y()?;
        Ok(ProofFingerprint {
            keyset_id: proof.keyset_id,
            amount: proof.amount.into(),
            y,
            c: proof.c,
            dleq: proof.dleq,
            witness: proof.witness,
        })
    }
}

pub fn fp_to_proof(fp: &ProofFingerprint, secret: cashu::secret::Secret) -> cashu::Proof {
    cashu::Proof {
        keyset_id: fp.keyset_id,
        amount: cashu::Amount::from(fp.amount),
        c: fp.c,
        dleq: fp.dleq.clone(),
        witness: fp.witness.clone(),
        secret,
    }
}
