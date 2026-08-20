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

/// Domain separation tag for the offline exchange redemption digest.
pub const DOMAIN_TAG_REDEEM: &[u8] = b"bcr/exchange/redeem/v1";

/// `SHA256(borsh(outputs))`. Order-preserving, unlike `fp_digest`: the mint returns
/// signatures positionally.
pub fn outputs_digest(outputs: &[cashu::BlindedMessage]) -> [u8; 32] {
    let mut bytes = Vec::new();
    wire_borsh::serialize_vecof_blindedmessage(outputs, &mut bytes)
        .expect("borsh serialization of blinded messages");
    Sha256::hash(&bytes).to_byte_array()
}

/// `SHA256(DOMAIN_TAG_REDEEM || alpha_id || exchange_digest || outputs_digest(outputs))`.
pub fn redemption_digest(
    alpha_id: &secp256k1::PublicKey,
    exchange_digest: &[u8; 32],
    outputs: &[cashu::BlindedMessage],
) -> [u8; 32] {
    let mut msg = Vec::with_capacity(DOMAIN_TAG_REDEEM.len() + 33 + 32 + 32);
    msg.extend_from_slice(DOMAIN_TAG_REDEEM);
    msg.extend_from_slice(&alpha_id.serialize());
    msg.extend_from_slice(exchange_digest);
    msg.extend_from_slice(&outputs_digest(outputs));
    Sha256::hash(&msg).to_byte_array()
}

/// The message the wallet signs; the digest is already domain-tagged.
pub fn redemption_message(redemption_digest: &[u8; 32]) -> Message {
    Message::from_digest(*redemption_digest)
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ExchangeError {
    #[error("schnorr signature verification failed: {0}")]
    Signature(#[from] secp256k1::Error),
    #[error("exchange digest does not match its fields")]
    DigestMismatch,
    #[error("redemption outputs do not total the amount claimable")]
    AmountMismatch,
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

    /// The wallet's half of this entry, as the broadcast an Alpha puts on
    /// its chain at close.
    pub fn into_broadcast(
        self,
        alpha_id: secp256k1::PublicKey,
        evidence_digest: [u8; 32],
    ) -> ExchangeBroadcast {
        let exchange_digest = self.exchange_digest(&alpha_id, &evidence_digest);
        ExchangeBroadcast {
            alpha_id,
            evidence_digest,
            exchange_digest,
            fingerprints: self.fingerprints,
            hashes: self.hashes,
            wallet_pk: self.wallet_pk,
            wallet_signature: self.wallet_signature,
        }
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

///--------------------------- Redeem Offline Exchange (Wallet -> Alpha)
/// Claims an exchange the alpha spent at close but never issued against. The authority
/// is the key the spend entry recorded, so the request names none of its own.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RedeemOfflineExchangeRequest {
    pub exchange_digest: [u8; 32],
    pub outputs: Vec<cashu::BlindedMessage>,
    /// Over the redemption digest.
    #[schema(value_type = String)]
    pub wallet_signature: secp256k1::schnorr::Signature,
}

impl RedeemOfflineExchangeRequest {
    pub fn new(
        alpha_id: &secp256k1::PublicKey,
        exchange_digest: [u8; 32],
        outputs: Vec<cashu::BlindedMessage>,
        wallet_kp: &secp256k1::Keypair,
    ) -> Self {
        let digest = redemption_digest(alpha_id, &exchange_digest, &outputs);
        Self {
            exchange_digest,
            outputs,
            wallet_signature: SECP256K1.sign_schnorr(&redemption_message(&digest), wallet_kp),
        }
    }

    /// Takes `amount` so no caller can authorise a redemption without checking value.
    pub fn verify(
        &self,
        alpha_id: &secp256k1::PublicKey,
        wallet_pk: &cashu::PublicKey,
        amount: cashu::Amount,
    ) -> Result<(), ExchangeError> {
        let total = cashu::Amount::try_sum(self.outputs.iter().map(|o| o.amount))
            .map_err(|_| ExchangeError::AmountMismatch)?;
        if total != amount {
            return Err(ExchangeError::AmountMismatch);
        }
        let digest = redemption_digest(alpha_id, &self.exchange_digest, &self.outputs);
        SECP256K1.verify_schnorr(
            &self.wallet_signature,
            &redemption_message(&digest),
            &wallet_pk.x_only_public_key(),
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RedeemOfflineExchangeResponse {
    pub signatures: Vec<cashu::BlindSignature>,
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

/// Shared with the other wire modules that embed an exchange.
#[cfg(test)]
pub mod tests_support {
    use super::*;
    use crate::core_tests;
    use bitcoin::secp256k1 as secp;

    pub fn sample_fingerprints(amounts: &[u64]) -> Vec<wire_keys::ProofFingerprint> {
        let (_, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts: Vec<cashu::Amount> = amounts.iter().map(|a| cashu::Amount::from(*a)).collect();
        core_tests::generate_random_ecash_proofs(&keyset, &amounts)
            .into_iter()
            .map(|p| wire_keys::ProofFingerprint::try_from(p).expect("fp"))
            .collect()
    }

    pub fn sample_hashes(n: u8) -> Vec<Sha256> {
        (0..n).map(|i| Sha256::hash(&[i])).collect()
    }

    pub fn wallet() -> secp::Keypair {
        secp::Keypair::new_global(&mut rand::thread_rng())
    }

    pub fn sample_outputs(amounts: &[u64]) -> Vec<cashu::BlindedMessage> {
        let (_, keyset) = core_tests::generate_random_ecash_keyset();
        let amounts: Vec<cashu::Amount> = amounts.iter().map(|a| cashu::Amount::from(*a)).collect();
        core_tests::generate_random_ecash_blindedmessages(keyset.id, &amounts)
            .into_iter()
            .map(|(msg, _, _)| msg)
            .collect()
    }

    pub fn sample_broadcast(wallet_kp: &secp::Keypair) -> ExchangeBroadcast {
        let alpha_id = wallet().public_key();
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
}

#[cfg(test)]
mod tests {
    use super::tests_support::*;
    use super::*;

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

    // A broadcast built from an entry carries the same digest and still verifies.
    #[test]
    fn entry_and_broadcast_agree_on_one_claim() {
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

        let broadcast = entry.into_broadcast(alpha_id, evidence);
        assert_eq!(broadcast.exchange_digest, digest);
        broadcast
            .verify()
            .expect("an entry's wallet half stands on its own");
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

    #[test]
    fn redemption_digest_binds_every_field() {
        let alpha = wallet().public_key();
        let exchange = [7u8; 32];
        let outputs = sample_outputs(&[1, 2]);
        let original = redemption_digest(&alpha, &exchange, &outputs);

        assert_ne!(
            redemption_digest(&wallet().public_key(), &exchange, &outputs),
            original
        );
        assert_ne!(redemption_digest(&alpha, &[8u8; 32], &outputs), original);
        assert_ne!(
            redemption_digest(&alpha, &exchange, &sample_outputs(&[1, 2])),
            original
        );

        // Same blinded secrets, different amount: the digest still has to move.
        let mut inflated = outputs.clone();
        inflated[0].amount = cashu::Amount::from(64_u64);
        assert_ne!(redemption_digest(&alpha, &exchange, &inflated), original);

        let mut rekeyed = outputs.clone();
        rekeyed[0].keyset_id = sample_outputs(&[1])[0].keyset_id;
        assert_ne!(redemption_digest(&alpha, &exchange, &rekeyed), original);
    }

    #[test]
    fn redemption_digest_is_order_sensitive() {
        let alpha = wallet().public_key();
        let exchange = [7u8; 32];
        let mut outputs = sample_outputs(&[1, 2]);
        let original = redemption_digest(&alpha, &exchange, &outputs);
        outputs.swap(0, 1);
        assert_ne!(redemption_digest(&alpha, &exchange, &outputs), original);
    }

    #[test]
    fn redemption_request_verifies_against_the_recorded_key() {
        let alpha = wallet().public_key();
        let wallet_kp = wallet();
        let wallet_pk: cashu::PublicKey = wallet_kp.public_key().into();
        let amount = cashu::Amount::from(3_u64);
        let req = RedeemOfflineExchangeRequest::new(
            &alpha,
            [7u8; 32],
            sample_outputs(&[1, 2]),
            &wallet_kp,
        );

        req.verify(&alpha, &wallet_pk, amount).expect("own claim");

        // Another wallet cannot claim a spend entry recorded for this one.
        let other: cashu::PublicKey = wallet().public_key().into();
        assert!(matches!(
            req.verify(&alpha, &other, amount),
            Err(ExchangeError::Signature(_))
        ));
        // Nor replayed at another alpha, or against another exchange of this one.
        assert!(
            req.verify(&wallet().public_key(), &wallet_pk, amount)
                .is_err()
        );
        let mut moved = req.clone();
        moved.exchange_digest = [9u8; 32];
        assert!(moved.verify(&alpha, &wallet_pk, amount).is_err());
    }

    #[test]
    fn redemption_totalling_more_than_claimable_is_refused() {
        let alpha = wallet().public_key();
        let wallet_kp = wallet();
        let wallet_pk: cashu::PublicKey = wallet_kp.public_key().into();
        let req = RedeemOfflineExchangeRequest::new(
            &alpha,
            [7u8; 32],
            sample_outputs(&[1, 2]),
            &wallet_kp,
        );

        // Correctly signed for its own outputs, but the entry owes less than they ask.
        assert!(matches!(
            req.verify(&alpha, &wallet_pk, cashu::Amount::from(2_u64)),
            Err(ExchangeError::AmountMismatch)
        ));
        // Overflowing outputs report a mismatch rather than panicking the mint.
        let mut overflowing = req.clone();
        overflowing.outputs[0].amount = cashu::Amount::from(u64::MAX);
        overflowing.outputs[1].amount = cashu::Amount::from(u64::MAX);
        assert!(matches!(
            overflowing.verify(&alpha, &wallet_pk, cashu::Amount::from(3_u64)),
            Err(ExchangeError::AmountMismatch)
        ));
    }

    #[test]
    fn redemption_request_json_roundtrip() {
        let alpha = wallet().public_key();
        let wallet_kp = wallet();
        let req = RedeemOfflineExchangeRequest::new(
            &alpha,
            [7u8; 32],
            sample_outputs(&[1, 2]),
            &wallet_kp,
        );
        let back: RedeemOfflineExchangeRequest =
            serde_json::from_str(&serde_json::to_string(&req).expect("ser")).expect("de");
        assert_eq!(back.exchange_digest, req.exchange_digest);
        assert_eq!(back.wallet_signature, req.wallet_signature);
        back.verify(
            &alpha,
            &wallet_kp.public_key().into(),
            cashu::Amount::from(3_u64),
        )
        .expect("survives the wire");
    }
}
