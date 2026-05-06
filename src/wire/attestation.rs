// ----- standard library imports
// ----- extra library imports
use bitcoin::hashes::{Hash, sha256::Hash as Sha256};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::wire::{
    borsh::{deserialize_from_str, serialize_as_str},
    keys::ProofFingerprint,
};

// ----- end imports

/// Domain separation tag for the Beta-issued attestation signature.
pub const DOMAIN_TAG_ATTEST: &[u8] = b"bcr/attest/issuance/v1";
/// Domain separation tag for the Beta verification response signature.
pub const DOMAIN_TAG_VERIFY: &[u8] = b"bcr/attest/verify/v1";

///--------------------------- Issuance Attestation Request (Wallet -> Beta)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IssuanceAttestationRequest {
    #[schema(value_type = String)]
    pub alpha_id: bitcoin::secp256k1::PublicKey,
    /// Inputs whose ancestry the Beta must attest to. `dleq` (with `r`) must
    /// be populated on every entry so the Beta can reblind `C_ <- C + r*K` and ensure C_ is signed correctly.
    pub inputs: Vec<ProofFingerprint>,
}

///--------------------------- Issuance Attestation
#[derive(
    Debug, Clone, Serialize, Deserialize, ToSchema, BorshSerialize, BorshDeserialize, PartialEq,
)]
pub struct IssuanceAttestation {
    #[schema(value_type = String)]
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub beta_id: bitcoin::secp256k1::PublicKey,
    /// Binds the attestation to this exact set of inputs.
    /// SHA256(borsh(canonical Vec<ProofFingerprint>)).
    pub fp_digest: [u8; 32],
    /// Opaque commitment to the inputs' stream coordinates `(h, i)`; only the
    /// attesting Beta can open it. HMAC(beta_secret, h_1 || i_1 || ... || h_n || i_n).
    pub coords_mac: [u8; 32],
    #[schema(value_type = String)]
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub signature: bitcoin::secp256k1::schnorr::Signature,
}

///--------------------------- Attestation Verify (Alpha -> Beta, POST /v1/attest/verify)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AttestationVerifyRequest {
    #[schema(value_type = String)]
    pub alpha_id: bitcoin::secp256k1::PublicKey,
    pub attestation: IssuanceAttestation,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AttestationVerifyResponse {
    pub found: bool,
    pub fp_digest: [u8; 32],
    pub coords_mac: [u8; 32],
    #[schema(value_type = String)]
    pub response_sig: bitcoin::secp256k1::schnorr::Signature,
}

/// Canonical form: only `(keyset_id, amount, y, c)` are kept. DLEQ is dropped
/// as Alpha doesn't have it
pub fn canonical_fingerprint(fp: &ProofFingerprint) -> ProofFingerprint {
    ProofFingerprint {
        keyset_id: fp.keyset_id,
        amount: fp.amount,
        y: fp.y,
        c: fp.c,
        dleq: None,
        witness: None,
    }
}

/// SHA256(borsh(canonical Vec<ProofFingerprint> sorted by `y`)). Sorting by
/// `y` makes the digest order-independent.
pub fn fp_digest(fps: &[ProofFingerprint]) -> [u8; 32] {
    let mut canonical: Vec<ProofFingerprint> = fps.iter().map(canonical_fingerprint).collect();
    canonical.sort_unstable_by(|a, b| a.y.to_bytes().cmp(&b.y.to_bytes()));
    let bytes = borsh::to_vec(&canonical).expect("borsh serialization of canonical fingerprints");
    Sha256::hash(&bytes).to_byte_array()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_tests;
    use bitcoin::secp256k1 as secp;

    fn sample_attestation() -> IssuanceAttestation {
        let keypair = secp::Keypair::new_global(&mut rand::thread_rng());
        let xonly = secp::XOnlyPublicKey::from_keypair(&keypair).0;
        let msg = secp::Message::from_digest([7u8; 32]);
        let signature = secp::global::SECP256K1.sign_schnorr(&msg, &keypair);
        secp::global::SECP256K1
            .verify_schnorr(&signature, &msg, &xonly)
            .expect("self-verify");
        IssuanceAttestation {
            beta_id: keypair.public_key(),
            fp_digest: [1u8; 32],
            coords_mac: [2u8; 32],
            signature,
        }
    }

    #[test]
    fn issuance_attestation_json_roundtrip() {
        let att = sample_attestation();
        let json = serde_json::to_string(&att).expect("serialize");
        let back: IssuanceAttestation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(att, back);
    }

    #[test]
    fn issuance_attestation_borsh_roundtrip() {
        let att = sample_attestation();
        let bytes = borsh::to_vec(&att).expect("borsh serialize");
        let back: IssuanceAttestation = borsh::from_slice(&bytes).expect("borsh deserialize");
        assert_eq!(att, back);
    }

    #[test]
    fn fp_digest_strips_dleq_and_is_stable() {
        use std::str::FromStr;
        let (_, keyset) = core_tests::generate_random_ecash_keyset();
        let proofs =
            core_tests::generate_random_ecash_proofs(&keyset, &[cashu::Amount::from(1u64)]);
        let proof = proofs.into_iter().next().unwrap();
        let with_dleq = ProofFingerprint::try_from(proof.clone()).expect("fp");
        // Forge a different DLEQ to prove canonicalization strips it.
        let mut tampered = with_dleq.clone();
        tampered.dleq = Some(cashu::ProofDleq {
            e: cashu::SecretKey::from_str(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            s: cashu::SecretKey::from_str(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
            r: cashu::SecretKey::from_str(
                "0000000000000000000000000000000000000000000000000000000000000003",
            )
            .unwrap(),
        });
        assert_eq!(fp_digest(&[with_dleq]), fp_digest(&[tampered]));
    }

    #[test]
    fn fp_digest_is_order_independent() {
        let (_, keyset) = core_tests::generate_random_ecash_keyset();
        let proofs = core_tests::generate_random_ecash_proofs(
            &keyset,
            &[
                cashu::Amount::from(1u64),
                cashu::Amount::from(2u64),
                cashu::Amount::from(4u64),
            ],
        );
        let mut fps: Vec<ProofFingerprint> = proofs
            .into_iter()
            .map(|p| ProofFingerprint::try_from(p).expect("fp"))
            .collect();
        let original = fp_digest(&fps);
        fps.reverse();
        assert_eq!(fp_digest(&fps), original);
        fps.swap(0, 1);
        assert_eq!(fp_digest(&fps), original);
    }
}
