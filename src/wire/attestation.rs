// ----- standard library imports
// ----- extra library imports
use bitcoin::hashes::{Hash, sha256::Hash as Sha256};
use bitcoin::secp256k1::{Message, PublicKey, SECP256K1};
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
    canonical.sort_unstable_by_key(|a| a.y.to_bytes());
    let bytes = borsh::to_vec(&canonical).expect("borsh serialization of canonical fingerprints");
    Sha256::hash(&bytes).to_byte_array()
}

#[derive(Debug, thiserror::Error)]
pub enum AttestationError {
    #[error("input has no derivable Y: {0}")]
    InvalidProof(#[from] cashu::nut00::Error),
    #[error("fp_digest mismatch")]
    DigestMismatch,
    #[error("attestation beta {0} is not part of alpha's cohort")]
    UnknownBeta(PublicKey),
    #[error("verify response signals not_found")]
    VerifyNotFound,
    #[error("schnorr signature verification failed: {0}")]
    Signature(#[from] bitcoin::secp256k1::Error),
}

pub fn project_to_fingerprints(
    inputs: &[cashu::Proof],
) -> Result<Vec<ProofFingerprint>, AttestationError> {
    inputs
        .iter()
        .cloned()
        .map(|p| ProofFingerprint::try_from(p).map_err(AttestationError::from))
        .collect()
}

/// `SHA256(DOMAIN_TAG_ATTEST || alpha_id || fp_digest || coords_mac)`.
pub fn attest_message(alpha_id: &PublicKey, fp_digest: &[u8; 32], coords_mac: &[u8; 32]) -> Sha256 {
    let mut msg = Vec::with_capacity(DOMAIN_TAG_ATTEST.len() + 33 + 32 + 32);
    msg.extend_from_slice(DOMAIN_TAG_ATTEST);
    msg.extend_from_slice(&alpha_id.serialize());
    msg.extend_from_slice(fp_digest);
    msg.extend_from_slice(coords_mac);
    Sha256::hash(&msg)
}

/// `SHA256(DOMAIN_TAG_VERIFY || alpha_id || fp_digest || found || coords_mac)`.
pub fn verify_message(
    alpha_id: &PublicKey,
    fp_digest: &[u8; 32],
    found: bool,
    coords_mac: &[u8; 32],
) -> Sha256 {
    let mut msg = Vec::with_capacity(DOMAIN_TAG_VERIFY.len() + 33 + 32 + 1 + 32);
    msg.extend_from_slice(DOMAIN_TAG_VERIFY);
    msg.extend_from_slice(&alpha_id.serialize());
    msg.extend_from_slice(fp_digest);
    msg.push(found as u8);
    msg.extend_from_slice(coords_mac);
    Sha256::hash(&msg)
}

pub fn verify_attestation_local(
    alpha_id: &PublicKey,
    inputs: &[cashu::Proof],
    attestation: &IssuanceAttestation,
    is_known_beta: impl FnOnce(&PublicKey) -> bool,
) -> Result<(), AttestationError> {
    if !is_known_beta(&attestation.beta_id) {
        return Err(AttestationError::UnknownBeta(attestation.beta_id));
    }
    let fps = project_to_fingerprints(inputs)?;
    let local = fp_digest(&fps);
    if local != attestation.fp_digest {
        return Err(AttestationError::DigestMismatch);
    }
    let msg_hash = attest_message(alpha_id, &attestation.fp_digest, &attestation.coords_mac);
    let secp_msg = Message::from_digest(*msg_hash.as_ref());
    SECP256K1.verify_schnorr(
        &attestation.signature,
        &secp_msg,
        &attestation.beta_id.x_only_public_key().0,
    )?;
    Ok(())
}

pub fn verify_attestation_response(
    alpha_id: &PublicKey,
    beta_id: &PublicKey,
    attestation: &IssuanceAttestation,
    response: &AttestationVerifyResponse,
) -> Result<(), AttestationError> {
    if response.fp_digest != attestation.fp_digest {
        return Err(AttestationError::DigestMismatch);
    }
    let msg_hash = verify_message(
        alpha_id,
        &response.fp_digest,
        response.found,
        &response.coords_mac,
    );
    let secp_msg = Message::from_digest(*msg_hash.as_ref());
    SECP256K1.verify_schnorr(
        &response.response_sig,
        &secp_msg,
        &beta_id.x_only_public_key().0,
    )?;
    if !response.found {
        return Err(AttestationError::VerifyNotFound);
    }
    Ok(())
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

    fn sample_inputs() -> Vec<cashu::Proof> {
        let (_, keyset) = core_tests::generate_random_ecash_keyset();
        core_tests::generate_random_ecash_proofs(
            &keyset,
            &[cashu::Amount::from(1u64), cashu::Amount::from(2u64)],
        )
    }

    fn make_attestation(
        beta: &secp::Keypair,
        alpha_id: &PublicKey,
        inputs: &[cashu::Proof],
    ) -> IssuanceAttestation {
        let fps = project_to_fingerprints(inputs).unwrap();
        let digest = fp_digest(&fps);
        let coords_mac = [9u8; 32];
        let msg_hash = attest_message(alpha_id, &digest, &coords_mac);
        let secp_msg = Message::from_digest(*msg_hash.as_ref());
        let signature = SECP256K1.sign_schnorr(&secp_msg, beta);
        IssuanceAttestation {
            beta_id: beta.public_key(),
            fp_digest: digest,
            coords_mac,
            signature,
        }
    }

    fn known_beta(beta: PublicKey) -> impl Fn(&PublicKey) -> bool {
        move |b| b == &beta
    }

    #[test]
    fn verify_attestation_local_happy_path() {
        let alpha = secp::Keypair::new_global(&mut rand::thread_rng());
        let beta = secp::Keypair::new_global(&mut rand::thread_rng());
        let inputs = sample_inputs();
        let att = make_attestation(&beta, &alpha.public_key(), &inputs);
        verify_attestation_local(
            &alpha.public_key(),
            &inputs,
            &att,
            known_beta(beta.public_key()),
        )
        .unwrap();
    }

    #[test]
    fn verify_attestation_local_tampered_inputs_rejected() {
        let alpha = secp::Keypair::new_global(&mut rand::thread_rng());
        let beta = secp::Keypair::new_global(&mut rand::thread_rng());
        let inputs = sample_inputs();
        let att = make_attestation(&beta, &alpha.public_key(), &inputs);
        let other = sample_inputs();
        let err = verify_attestation_local(
            &alpha.public_key(),
            &other,
            &att,
            known_beta(beta.public_key()),
        )
        .unwrap_err();
        assert!(matches!(err, AttestationError::DigestMismatch));
    }

    #[test]
    fn verify_attestation_local_unknown_beta_rejected() {
        let alpha = secp::Keypair::new_global(&mut rand::thread_rng());
        let beta = secp::Keypair::new_global(&mut rand::thread_rng());
        let inputs = sample_inputs();
        let att = make_attestation(&beta, &alpha.public_key(), &inputs);
        let other = secp::Keypair::new_global(&mut rand::thread_rng());
        let err = verify_attestation_local(
            &alpha.public_key(),
            &inputs,
            &att,
            known_beta(other.public_key()),
        )
        .unwrap_err();
        assert!(matches!(err, AttestationError::UnknownBeta(_)));
    }

    #[test]
    fn verify_attestation_response_round_trip() {
        let alpha = secp::Keypair::new_global(&mut rand::thread_rng());
        let beta = secp::Keypair::new_global(&mut rand::thread_rng());
        let inputs = sample_inputs();
        let att = make_attestation(&beta, &alpha.public_key(), &inputs);
        let coords_mac = att.coords_mac;
        let msg_hash = verify_message(&alpha.public_key(), &att.fp_digest, true, &coords_mac);
        let secp_msg = Message::from_digest(*msg_hash.as_ref());
        let response_sig = SECP256K1.sign_schnorr(&secp_msg, &beta);
        let response = AttestationVerifyResponse {
            found: true,
            fp_digest: att.fp_digest,
            coords_mac,
            response_sig,
        };
        verify_attestation_response(&alpha.public_key(), &beta.public_key(), &att, &response)
            .unwrap();
    }

    #[test]
    fn verify_attestation_response_not_found_rejected() {
        let alpha = secp::Keypair::new_global(&mut rand::thread_rng());
        let beta = secp::Keypair::new_global(&mut rand::thread_rng());
        let inputs = sample_inputs();
        let att = make_attestation(&beta, &alpha.public_key(), &inputs);
        let zero = [0u8; 32];
        let msg_hash = verify_message(&alpha.public_key(), &att.fp_digest, false, &zero);
        let secp_msg = Message::from_digest(*msg_hash.as_ref());
        let response_sig = SECP256K1.sign_schnorr(&secp_msg, &beta);
        let response = AttestationVerifyResponse {
            found: false,
            fp_digest: att.fp_digest,
            coords_mac: zero,
            response_sig,
        };
        let err =
            verify_attestation_response(&alpha.public_key(), &beta.public_key(), &att, &response)
                .unwrap_err();
        assert!(matches!(err, AttestationError::VerifyNotFound));
    }
}
