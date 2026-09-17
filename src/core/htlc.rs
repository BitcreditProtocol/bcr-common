// ----- standard library imports
// ----- extra library imports
use bitcoin::hashes::{Hash, sha256::Hash as Sha256};
use bitcoin::hex::FromHex;
use cashu::nut14 as cdk14;
// ----- local imports
use crate::core::signature::ECashSignatureResult;

// ----- end imports

/// Hash lock and side conditions of an HTLC proof; any other secret is an error.
pub fn htlc_lock(proof: &cashu::Proof) -> ECashSignatureResult<(Sha256, cashu::Conditions)> {
    let spending: cashu::SpendingConditions = (&proof.secret)
        .try_into()
        .map_err(|_| cdk14::Error::IncorrectSecretKind)?;
    match spending {
        cashu::SpendingConditions::HTLCConditions {
            data,
            conditions: Some(conditions),
        } => Ok((data, conditions)),
        _ => Err(cdk14::Error::IncorrectSecretKind.into()),
    }
}

/// Inter-mint exchange HTLC: one recipient, one refund key, a single signature each. This is the
/// shape the alpha-lock validator accepts; the refund key is the issuing mint.
pub fn exchange_htlc(
    hash_lock: Sha256,
    locktime: u64,
    recipient: cashu::PublicKey,
    refund: cashu::PublicKey,
) -> ECashSignatureResult<cashu::SpendingConditions> {
    let conditions = cashu::Conditions::new(
        Some(locktime),
        Some(vec![recipient]),
        Some(vec![refund]),
        Some(1),
        None,
        Some(1),
    )?;
    Ok(cashu::SpendingConditions::new_htlc_hash(
        &hash_lock.to_string(),
        Some(conditions),
    )?)
}

/// Locktime left for the next hop, `None` when the input does not leave a full margin.
pub fn hop_locktime(locktime: u64, now: u64, margin: u64) -> Option<u64> {
    locktime.checked_sub(margin).filter(|next| *next >= now)
}

/// Online-exchange hash lock: sha256 over the 32 bytes of a hex preimage.
pub fn online_hash_lock(preimage: &str) -> Option<Sha256> {
    <[u8; 32]>::from_hex(preimage)
        .ok()
        .map(|bytes| Sha256::hash(&bytes))
}

/// Offline-exchange hash lock: sha256 over the ASCII bytes of the alpha proof's secret, which is
/// the preimage the wallet reveals.
pub fn offline_hash_lock(preimage: &str) -> Sha256 {
    Sha256::hash(preimage.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::signature::{offline_htlc_secret, verify_offline_exchange_htlc};
    use crate::core_tests;
    use bitcoin::secp256k1 as secp;

    fn proof_with(secret: cashu::secret::Secret) -> cashu::Proof {
        let (_, keyset) = core_tests::generate_random_ecash_keyset();
        let kp = secp::Keypair::new_global(&mut rand::thread_rng());
        let pk: cashu::PublicKey = cashu::SecretKey::from(kp.secret_key()).public_key();
        cashu::Proof::new(cashu::Amount::from(1u64), keyset.id.into(), secret, pk)
    }

    fn secret_of(spending: cashu::SpendingConditions) -> cashu::secret::Secret {
        spending.try_into().expect("secret")
    }

    #[test]
    fn htlc_lock_round_trips_exchange_htlc() {
        let hash = offline_hash_lock("preimage");
        let recipient = cashu::SecretKey::generate().public_key();
        let refund = cashu::SecretKey::generate().public_key();
        let locktime = cashu::util::unix_time() + 3600;
        let spending = exchange_htlc(hash, locktime, recipient, refund).expect("htlc");
        let (lock, conditions) = htlc_lock(&proof_with(secret_of(spending))).expect("htlc lock");
        assert_eq!(lock, hash);
        assert_eq!(conditions.locktime, Some(locktime));
        assert_eq!(conditions.pubkeys, Some(vec![recipient]));
        assert_eq!(conditions.refund_keys, Some(vec![refund]));
        assert_eq!(conditions.num_sigs, Some(1));
        assert_eq!(conditions.num_sigs_refund, Some(1));
    }

    #[test]
    fn htlc_lock_rejects_non_exchange_secrets() {
        let pk = cashu::SecretKey::generate().public_key();
        let hash = offline_hash_lock("preimage");
        let plain = cashu::secret::Secret::new("plain");
        let p2pk = secret_of(cashu::SpendingConditions::new_p2pk(pk, None));
        let bare_htlc = secret_of(
            cashu::SpendingConditions::new_htlc_hash(&hash.to_string(), None).expect("htlc"),
        );
        for secret in [plain, p2pk, bare_htlc] {
            assert!(htlc_lock(&proof_with(secret)).is_err());
        }
    }

    #[test]
    fn hop_locktime_requires_a_full_margin() {
        let (now, margin) = (1_700_000_000, 600);
        assert_eq!(hop_locktime(now + margin, now, margin), Some(now));
        assert_eq!(hop_locktime(now + margin + 1, now, margin), Some(now + 1));
        assert_eq!(hop_locktime(now + margin - 1, now, margin), None);
        assert_eq!(hop_locktime(margin - 1, now, margin), None);
    }

    #[test]
    fn online_hash_lock_needs_32_hex_bytes() {
        let bytes = [7u8; 32];
        let hex = bitcoin::hex::DisplayHex::to_lower_hex_string(&bytes[..]);
        assert_eq!(online_hash_lock(&hex), Some(Sha256::hash(&bytes)));
        assert_eq!(online_hash_lock(&hex[..62]), None);
        assert_eq!(online_hash_lock("not hex at all"), None);
    }

    /// The offline verifier and the mints hash the preimage the same way.
    #[test]
    fn offline_hash_lock_matches_the_offline_verifier() {
        let kp = secp::Keypair::new_global(&mut rand::thread_rng());
        let wallet: cashu::SecretKey = kp.secret_key().into();
        let refund = cashu::SecretKey::generate().public_key();
        let preimage = "the alpha proof secret";
        let spending = exchange_htlc(
            offline_hash_lock(preimage),
            cashu::util::unix_time() + 3600,
            wallet.public_key(),
            refund,
        )
        .expect("htlc");
        let mut proof = proof_with(offline_htlc_secret(spending).expect("tagged secret"));
        let signature = wallet.sign(&proof.secret.to_bytes()).expect("sign");
        proof.witness = Some(cashu::Witness::HTLCWitness(cashu::HTLCWitness {
            preimage: preimage.to_string(),
            signatures: Some(vec![signature.to_string()]),
        }));
        verify_offline_exchange_htlc(&proof).expect("valid offline htlc");
        assert_eq!(
            htlc_lock(&proof).expect("lock").0,
            offline_hash_lock(preimage)
        );
    }
}
