// ----- standard library imports
// ----- extra library imports
use cashu::{Amount, Id, PublicKey, nut02::ShortKeysetId, secret::Secret};
use serde::{Deserialize, Serialize};
// ----- local modules
use crate::ecash::{Proof, Proofs};

// ----- end imports

/// Proofs of a single keyset, as carried by a token
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenV4Token {
    /// `Keyset id`
    #[serde(rename = "i", with = "crate::wallet::cbor")]
    pub keyset_id: ShortKeysetId,
    /// Proofs
    #[serde(rename = "p")]
    pub proofs: Vec<ProofV4>,
}

impl TokenV4Token {
    /// Create new [`TokenV4Token`]
    pub fn new(keyset_id: Id, proofs: Proofs) -> Self {
        Self {
            keyset_id: ShortKeysetId::from(keyset_id),
            proofs: proofs.into_iter().map(Into::into).collect(),
        }
    }
}

/// Proof as carried by a token: cashu's `ProofV4` with an absent dleq omitted
/// from the cbor map instead of encoded as an explicit null. Tokens that already
/// omit it round-trip byte for byte; tokens carrying `d: null` are normalised to
/// the shorter form on re-encode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofV4 {
    /// Amount
    #[serde(rename = "a")]
    pub amount: Amount,
    /// Secret message
    #[serde(rename = "s")]
    pub secret: Secret,
    /// Unblinded signature
    #[serde(with = "crate::wallet::cbor")]
    pub c: PublicKey,
    /// Witness
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub witness: Option<cashu::Witness>,
    /// DLEQ Proof
    #[serde(rename = "d", default, skip_serializing_if = "Option::is_none")]
    pub dleq: Option<cashu::ProofDleq>,
    /// P2BK Ephemeral Public Key (NUT-28)
    #[serde(rename = "pe", default, skip_serializing_if = "Option::is_none")]
    pub p2pk_e: Option<PublicKey>,
}

impl ProofV4 {
    /// `y`, available without resolving the short keyset id
    pub fn y(&self) -> crate::ecash::Result<PublicKey> {
        crate::ecash::y_of(&self.secret)
    }

    /// [`ProofV4`] into [`Proof`]
    pub fn into_proof(&self, keyset_id: &Id) -> Proof {
        Proof {
            amount: self.amount,
            keyset_id: *keyset_id,
            secret: self.secret.clone(),
            c: self.c,
            witness: self.witness.clone(),
            dleq: self.dleq.clone(),
            p2pk_e: self.p2pk_e,
        }
    }
}

impl From<Proof> for ProofV4 {
    fn from(proof: Proof) -> Self {
        let Proof {
            amount,
            secret,
            c,
            witness,
            dleq,
            p2pk_e,
            ..
        } = proof;
        Self {
            amount,
            secret,
            c,
            witness,
            dleq,
            p2pk_e,
        }
    }
}
