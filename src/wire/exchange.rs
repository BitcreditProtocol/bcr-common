// ----- standard library imports
// ----- extra library imports
use bitcoin::hashes::{Hash, sha256::Hash as Sha256};
use bitcoin::secp256k1::{self, Message, SECP256K1};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::wire::{attestation::fp_digest, borsh as wire_borsh, keys as wire_keys};

// ----- end imports

/// Domain separation tag for the offline exchange digest.
pub const DOMAIN_TAG_EXCHANGE: &[u8] = b"bcr/exchange/offline/v1";

/// `SHA256(DOMAIN_TAG_EXCHANGE || alpha_id || evidence_digest || fp_digest(fps)
/// || sorted hash locks || wallet_pk)`. Order-independent over fingerprints and
/// hashes; excludes the substitute key (any substitute may complete it) and the
/// frozen tip (`evidence_digest` pins it).
pub fn exchange_digest(
    alpha_id: &secp256k1::PublicKey,
    evidence_digest: &[u8; 32],
    fingerprints: &[wire_keys::ProofFingerprint],
    hashes: &[Sha256],
    wallet_pk: &cashu::PublicKey,
) -> [u8; 32] {
    let fps = fp_digest(fingerprints);
    let mut sorted: Vec<[u8; 32]> = hashes.iter().map(|h| h.to_byte_array()).collect();
    sorted.sort_unstable();
    let mut msg =
        Vec::with_capacity(DOMAIN_TAG_EXCHANGE.len() + 33 + 32 + 32 + 32 * sorted.len() + 33);
    msg.extend_from_slice(DOMAIN_TAG_EXCHANGE);
    msg.extend_from_slice(&alpha_id.serialize());
    msg.extend_from_slice(evidence_digest);
    msg.extend_from_slice(&fps);
    for hash in &sorted {
        msg.extend_from_slice(hash);
    }
    msg.extend_from_slice(&wallet_pk.to_bytes());
    Sha256::hash(&msg).to_byte_array()
}

/// The message both parties sign; the digest is already domain-tagged.
pub fn exchange_message(exchange_digest: &[u8; 32]) -> Message {
    Message::from_digest(*exchange_digest)
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ExchangeError {
    #[error("schnorr signature verification failed: {0}")]
    Signature(#[from] secp256k1::Error),
    #[error("exchange digest does not match its fields")]
    DigestMismatch,
}

///--------------------------- Online ExchangeRequest
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct OnlineExchangeRequest {
    pub proofs: Vec<cashu::Proof>,
    #[schema(value_type = Vec<String>)]
    pub exchange_path: Vec<secp256k1::PublicKey>,
}

///--------------------------- Online ExchangeResponse
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct OnlineExchangeResponse {
    pub proofs: Vec<cashu::Proof>,
}

///--------------------------- Offline ExchangeRequest (Wallet -> Substitute)
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct OfflineExchangeRequest {
    pub fingerprints: Vec<wire_keys::ProofFingerprint>,
    #[schema(value_type = Vec<String>)]
    pub hashes: Vec<bitcoin::hashes::sha256::Hash>,
    pub wallet_pk: cashu::PublicKey,
    /// Over the exchange digest; only wallet-signed reports can spend its proofs.
    #[schema(value_type = String)]
    pub wallet_signature: secp256k1::schnorr::Signature,
}

///--------------------------- Exchange Broadcast (Substitute -> Alpha's Betas)
/// Fire-and-forget. Grants nothing, but gates reopening: a Beta withholds its
/// lease ack until every exchange it received is spent on Alpha's chain. Each
/// Beta enforces only what it saw, so no quorum is needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeBroadcast {
    pub alpha_id: secp256k1::PublicKey,
    pub evidence_digest: [u8; 32],
    pub exchange_digest: [u8; 32],
    pub fingerprints: Vec<wire_keys::ProofFingerprint>,
    pub hashes: Vec<Sha256>,
    pub wallet_pk: cashu::PublicKey,
    pub wallet_signature: secp256k1::schnorr::Signature,
}

impl ExchangeBroadcast {
    /// Self-contained: the broadcast carries everything its digest commits to.
    pub fn verify(&self) -> Result<(), ExchangeError> {
        let expected = exchange_digest(
            &self.alpha_id,
            &self.evidence_digest,
            &self.fingerprints,
            &self.hashes,
            &self.wallet_pk,
        );
        if expected != self.exchange_digest {
            return Err(ExchangeError::DigestMismatch);
        }
        SECP256K1.verify_schnorr(
            &self.wallet_signature,
            &exchange_message(&self.exchange_digest),
            &self.wallet_pk.x_only_public_key(),
        )?;
        Ok(())
    }
}

///--------------------------- Exchange Entry (Substitute -> Alpha at close)
/// One exchange, signed by both wallet and substitute. Alpha admits a spend
/// only from a wallet-signed exchange, so no unsigned report can burn anyone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeEntry {
    pub fingerprints: Vec<wire_keys::ProofFingerprint>,
    pub hashes: Vec<Sha256>,
    pub wallet_pk: cashu::PublicKey,
    /// Over the exchange digest.
    pub wallet_signature: secp256k1::schnorr::Signature,
    /// Over the exchange digest.
    pub substitute_signature: secp256k1::schnorr::Signature,
}

impl ExchangeEntry {
    /// The digest this entry's signatures commit to, under the given outage.
    pub fn exchange_digest(
        &self,
        alpha_id: &secp256k1::PublicKey,
        evidence_digest: &[u8; 32],
    ) -> [u8; 32] {
        exchange_digest(
            alpha_id,
            evidence_digest,
            &self.fingerprints,
            &self.hashes,
            &self.wallet_pk,
        )
    }

    /// Verifies both signatures against the digest recomputed from the entry's
    /// own fields, so no field can be swapped after signing.
    pub fn verify(
        &self,
        alpha_id: &secp256k1::PublicKey,
        evidence_digest: &[u8; 32],
        substitute_id: &secp256k1::PublicKey,
    ) -> Result<[u8; 32], ExchangeError> {
        let digest = self.exchange_digest(alpha_id, evidence_digest);
        let msg = exchange_message(&digest);
        SECP256K1.verify_schnorr(
            &self.wallet_signature,
            &msg,
            &self.wallet_pk.x_only_public_key(),
        )?;
        SECP256K1.verify_schnorr(
            &self.substitute_signature,
            &msg,
            &substitute_id.x_only_public_key().0,
        )?;
        Ok(digest)
    }
}

///--------------------------- Record Offline Exchange (Substitute mint -> its node)
/// The digests the mint issues under, returned once the node has verified and
/// broadcast the exchange.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RecordOfflineExchangeResponse {
    pub evidence_digest: [u8; 32],
    pub exchange_digest: [u8; 32],
}

///--------------------------- Offline ExchangePayload
#[derive(Debug, Clone, borsh::BorshSerialize, borsh::BorshDeserialize)]
pub struct OfflineExchangePayload {
    #[borsh(
        serialize_with = "wire_borsh::serialize_vecof_cdkproof",
        deserialize_with = "wire_borsh::deserialize_vecof_cdkproof"
    )]
    pub proofs: Vec<cashu::Proof>,
}

///--------------------------- Offline ExchangeResponse
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OfflineExchangeResponse {
    pub content: String, // b64 borsh-serialized OfflineExchangePayload
    #[schema(value_type = String)]
    pub signature: bitcoin::secp256k1::schnorr::Signature,
}

///--------------------------- HtlcSwapAttemptRequest
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct HtlcSwapAttemptRequest {
    pub preimage: String,
}

///--------------------------- RequestToMintFromForeignECash
#[derive(Debug, borsh::BorshSerialize, borsh::BorshDeserialize, ToSchema)]
pub struct RequestToMintFromForeignECashPayload {
    pub foreign_amount_sat: u64,
    pub nonce: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequestToMintFromForeignECash {
    pub payload: String, // b64 borsh payload
    #[schema(value_type = String)]
    pub signature: secp256k1::schnorr::Signature,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_tests;
    use bitcoin::secp256k1 as secp;

    fn sample_fingerprints(amounts: &[u64]) -> Vec<wire_keys::ProofFingerprint> {
        let (_, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts: Vec<cashu::Amount> = amounts.iter().map(|a| cashu::Amount::from(*a)).collect();
        core_tests::generate_random_ecash_proofs(&keyset, &amounts)
            .into_iter()
            .map(|p| wire_keys::ProofFingerprint::try_from(p).expect("fp"))
            .collect()
    }

    fn sample_hashes(n: u8) -> Vec<Sha256> {
        (0..n).map(|i| Sha256::hash(&[i])).collect()
    }

    fn wallet() -> secp::Keypair {
        secp::Keypair::new_global(&mut rand::thread_rng())
    }

    #[test]
    fn exchange_digest_is_order_independent() {
        let alpha = wallet().public_key();
        let evidence = [3u8; 32];
        let mut fps = sample_fingerprints(&[1, 2, 4]);
        let mut hashes = sample_hashes(3);
        let wpk: cashu::PublicKey = wallet().public_key().into();
        let original = exchange_digest(&alpha, &evidence, &fps, &hashes, &wpk);
        fps.reverse();
        hashes.swap(0, 2);
        assert_eq!(
            exchange_digest(&alpha, &evidence, &fps, &hashes, &wpk),
            original
        );
    }

    #[test]
    fn exchange_digest_binds_every_field() {
        let alpha = wallet().public_key();
        let evidence = [3u8; 32];
        let fps = sample_fingerprints(&[1, 2]);
        let hashes = sample_hashes(2);
        let wpk: cashu::PublicKey = wallet().public_key().into();
        let original = exchange_digest(&alpha, &evidence, &fps, &hashes, &wpk);

        assert_ne!(
            exchange_digest(&wallet().public_key(), &evidence, &fps, &hashes, &wpk),
            original
        );
        assert_ne!(
            exchange_digest(&alpha, &[4u8; 32], &fps, &hashes, &wpk),
            original
        );
        assert_ne!(
            exchange_digest(
                &alpha,
                &evidence,
                &sample_fingerprints(&[1, 2]),
                &hashes,
                &wpk
            ),
            original
        );
        assert_ne!(
            exchange_digest(&alpha, &evidence, &fps, &sample_hashes(3), &wpk),
            original
        );
        assert_ne!(
            exchange_digest(
                &alpha,
                &evidence,
                &fps,
                &hashes,
                &wallet().public_key().into()
            ),
            original
        );
    }

    #[test]
    fn exchange_digest_is_dleq_independent() {
        let alpha = wallet().public_key();
        let evidence = [3u8; 32];
        let fps = sample_fingerprints(&[1]);
        let hashes = sample_hashes(1);
        let wpk: cashu::PublicKey = wallet().public_key().into();
        let original = exchange_digest(&alpha, &evidence, &fps, &hashes, &wpk);
        let mut stripped = fps.clone();
        stripped[0].dleq = None;
        assert_eq!(
            exchange_digest(&alpha, &evidence, &stripped, &hashes, &wpk),
            original
        );
    }

    fn sample_broadcast(wallet_kp: &secp::Keypair) -> ExchangeBroadcast {
        let alpha_id = secp::Keypair::new_global(&mut rand::thread_rng()).public_key();
        let evidence_digest = [3u8; 32];
        let fingerprints = sample_fingerprints(&[8]);
        let hashes = sample_hashes(1);
        let wallet_pk: cashu::PublicKey = wallet_kp.public_key().into();
        let digest = exchange_digest(
            &alpha_id,
            &evidence_digest,
            &fingerprints,
            &hashes,
            &wallet_pk,
        );
        ExchangeBroadcast {
            alpha_id,
            evidence_digest,
            exchange_digest: digest,
            fingerprints,
            hashes,
            wallet_pk,
            wallet_signature: SECP256K1.sign_schnorr(&exchange_message(&digest), wallet_kp),
        }
    }

    #[test]
    fn broadcast_is_self_verifying() {
        let wallet_kp = wallet();
        let broadcast = sample_broadcast(&wallet_kp);
        broadcast.verify().expect("authentic broadcast verifies");

        // A substitute cannot swap in another redemption key ...
        let mut swapped = broadcast.clone();
        swapped.wallet_pk = wallet().public_key().into();
        assert!(swapped.verify().is_err());

        // ... nor name proofs the wallet never signed for.
        let mut foreign = broadcast.clone();
        foreign.fingerprints = sample_fingerprints(&[8]);
        assert_eq!(foreign.verify(), Err(ExchangeError::DigestMismatch));

        // A forged digest fails before any signature is consulted.
        let mut forged = broadcast.clone();
        forged.exchange_digest = [9u8; 32];
        assert_eq!(forged.verify(), Err(ExchangeError::DigestMismatch));
    }

    #[test]
    fn entry_requires_both_signatures() {
        let wallet_kp = wallet();
        let substitute = wallet();
        let alpha_id = wallet().public_key();
        let evidence = [3u8; 32];
        let fingerprints = sample_fingerprints(&[8]);
        let hashes = sample_hashes(1);
        let wallet_pk: cashu::PublicKey = wallet_kp.public_key().into();
        let digest = exchange_digest(&alpha_id, &evidence, &fingerprints, &hashes, &wallet_pk);
        let msg = exchange_message(&digest);
        let entry = ExchangeEntry {
            fingerprints,
            hashes,
            wallet_pk,
            wallet_signature: SECP256K1.sign_schnorr(&msg, &wallet_kp),
            substitute_signature: SECP256K1.sign_schnorr(&msg, &substitute),
        };
        let verified = entry
            .verify(&alpha_id, &evidence, &substitute.public_key())
            .expect("dual-signed entry verifies");
        assert_eq!(verified, digest);

        // The wrong substitute cannot claim the entry as its own.
        assert!(
            entry
                .verify(&alpha_id, &evidence, &wallet().public_key())
                .is_err()
        );

        // A replay under another outage yields a different digest; nothing verifies.
        assert!(
            entry
                .verify(&alpha_id, &[7u8; 32], &substitute.public_key())
                .is_err()
        );

        // Marking foreign proofs spent needs a wallet signature it never gave.
        let mut foreign = entry.clone();
        foreign.fingerprints = sample_fingerprints(&[8]);
        assert!(
            foreign
                .verify(&alpha_id, &evidence, &substitute.public_key())
                .is_err()
        );
    }

    #[test]
    fn offline_exchange_request_json_roundtrip() {
        let wallet_kp = wallet();
        let broadcast = sample_broadcast(&wallet_kp);
        let request = OfflineExchangeRequest {
            fingerprints: broadcast.fingerprints.clone(),
            hashes: broadcast.hashes.clone(),
            wallet_pk: broadcast.wallet_pk,
            wallet_signature: broadcast.wallet_signature,
        };
        let json = serde_json::to_string(&request).expect("serialize");
        let back: OfflineExchangeRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.wallet_signature, request.wallet_signature);
        assert_eq!(back.fingerprints.len(), request.fingerprints.len());
    }

    #[test]
    fn broadcast_and_entry_cbor_roundtrip() {
        let wallet_kp = wallet();
        let broadcast = sample_broadcast(&wallet_kp);
        let mut bytes = Vec::new();
        ciborium::into_writer(&broadcast, &mut bytes).expect("serialize");
        let back: ExchangeBroadcast = ciborium::from_reader(bytes.as_slice()).expect("deserialize");
        back.verify()
            .expect("round-tripped broadcast still verifies");
        assert_eq!(back.exchange_digest, broadcast.exchange_digest);
    }
}
