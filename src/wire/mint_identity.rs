//! Purpose-bound proof of control of a Mint's eBill node key.
use std::str::FromStr;

use bitcoin::{
    hashes::{Hash, sha256},
    secp256k1::{Keypair, Message, SECP256K1, schnorr::Signature},
};
use serde::{Deserialize, Serialize};

use crate::core::NodeId;

pub const MINT_IDENTITY_SCHEMA: &str = "bitcredit-mint-identity-v1";
pub const MINT_IDENTITY_PATH: &str = "/v1/ebill/mint-identity";
pub const MINT_IDENTITY_CLOCK_SKEW_SECONDS: u64 = 300;

/// The only caller-controlled signing input: exactly 32 bytes of lowercase hex.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MintIdentityQuery {
    pub challenge: String,
}

pub fn valid_challenge(challenge: &str) -> bool {
    challenge.len() == 64
        && challenge
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// `issuedAt` is an integer Unix timestamp in seconds; `signature` is BIP340 hex.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MintIdentity {
    pub schema: String,
    pub network: String,
    pub node_id: NodeId,
    pub issued_at: i64,
    pub challenge: String,
    pub signature: String,
}

impl MintIdentity {
    /// UTF-8, no BOM, decimal Unix seconds, and a trailing LF after every line.
    pub fn canonical_text(&self) -> String {
        format!(
            "{}\n{}\n{}\n{}\n{}\n",
            self.schema, self.network, self.node_id, self.challenge, self.issued_at
        )
    }

    /// Never signs arbitrary text or a caller-supplied schema, key, network or time.
    /// The service supplies its persisted node key and clock.
    pub fn sign(
        node_id: NodeId,
        challenge: String,
        issued_at: i64,
        keys: &Keypair,
    ) -> Option<Self> {
        if !valid_challenge(&challenge) || node_id.pub_key() != keys.public_key() {
            return None;
        }
        let mut proof = Self {
            schema: MINT_IDENTITY_SCHEMA.to_owned(),
            network: node_id.network().to_string(),
            node_id,
            issued_at,
            challenge,
            signature: String::new(),
        };
        let hash = sha256::Hash::hash(proof.canonical_text().as_bytes());
        proof.signature = SECP256K1
            .sign_schnorr_no_aux_rand(&Message::from_digest(hash.to_byte_array()), keys)
            .to_string();
        Some(proof)
    }

    pub fn verify(&self, expected: &NodeId, challenge: &str, now: i64) -> bool {
        if self.schema != MINT_IDENTITY_SCHEMA
            || self.network != expected.network().to_string()
            || &self.node_id != expected
            || !valid_challenge(challenge)
            || self.challenge != challenge
            || self.issued_at.abs_diff(now) > MINT_IDENTITY_CLOCK_SKEW_SECONDS
        {
            return false;
        }
        let Ok(signature) = Signature::from_str(&self.signature) else {
            return false;
        };
        let hash = sha256::Hash::hash(self.canonical_text().as_bytes());
        SECP256K1
            .verify_schnorr(
                &signature,
                &Message::from_digest(hash.to_byte_array()),
                &expected.pub_key().x_only_public_key().0,
            )
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::{Network, secp256k1::SecretKey};

    #[test]
    fn mint_identity_round_trip_and_canonical_contract() {
        let keys = Keypair::from_secret_key(SECP256K1, &SecretKey::from_slice(&[1; 32]).unwrap());
        let node = NodeId::new(keys.public_key(), Network::Regtest);
        let challenge = "ab".repeat(32);
        let proof = MintIdentity::sign(node.clone(), challenge.clone(), 1700000000, &keys).unwrap();
        assert_eq!(
            proof.canonical_text(),
            format!("bitcredit-mint-identity-v1\nregtest\n{node}\n{challenge}\n1700000000\n")
        );
        assert!(proof.verify(&node, &challenge, 1700000000));
        assert!(proof.verify(&node, &challenge, 1700000300));
        assert!(!proof.verify(&node, &challenge, 1700000301));
        assert!(!proof.verify(&node, &challenge, 1699999699));
        assert!(!proof.verify(&node, &"cd".repeat(32), 1700000000));
        let json = serde_json::to_value(&proof).unwrap();
        assert!(json.get("nodeId").is_some());
        assert_eq!(json["issuedAt"], 1700000000_i64);
        let decoded: MintIdentity = serde_json::from_value(json).unwrap();
        assert!(decoded.verify(&node, &challenge, 1700000000));
        let mut changed = proof.clone();
        changed.signature = "00".repeat(64);
        assert!(!changed.verify(&node, &challenge, 1700000000));
        let mut changed = proof;
        changed.schema = "another-purpose".to_owned();
        assert!(!changed.verify(&node, &challenge, 1700000000));
    }

    #[test]
    fn mint_identity_challenges_are_strict() {
        assert!(valid_challenge(&"0".repeat(64)));
        for bad in [
            "".to_owned(),
            "a".repeat(63),
            "a".repeat(65),
            "A".repeat(64),
            "g".repeat(64),
            format!("{}\n", "0".repeat(63)),
        ] {
            assert!(!valid_challenge(&bad));
        }
    }
}
