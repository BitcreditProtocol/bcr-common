// ----- standard library imports
// ----- extra library imports
use bitcoin::hashes::{Hash, sha256::Hash as Sha256};
use bitcoin::hex::FromHex;
use cashu::nut14 as cdk14;
use thiserror::Error;
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

/// Wallet HTLC of a multi-hop exchange: every mint on the path signs, the wallet alone refunds.
pub fn wallet_htlc(
    hash_lock: Sha256,
    locktime: u64,
    key_locks: Vec<cashu::PublicKey>,
    refund: cashu::PublicKey,
) -> ECashSignatureResult<cashu::SpendingConditions> {
    let n = key_locks.len() as u64;
    let conditions = cashu::Conditions::new(
        Some(locktime),
        Some(key_locks),
        Some(vec![refund]),
        Some(n),
        None,
        Some(1),
    )?;
    Ok(cashu::SpendingConditions::new_htlc_hash(
        &hash_lock.to_string(),
        Some(conditions),
    )?)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LockError {
    #[error("missing proofs")]
    NoProofs,
    #[error("missing HTLC spending conditions")]
    MissingConditions,
    #[error("invalid lock time")]
    LockTime,
    #[error("invalid number of signatures")]
    NumSigs,
    #[error("invalid pubkeys")]
    Pubkeys,
    #[error("invalid refund key")]
    RefundKey,
    #[error("inconsistent spending conditions")]
    Inconsistent,
}

/// Checks a [`wallet_htlc`] that names `node` on its path, leaving every hop `margin` seconds.
/// Returns its locktime and the wallet's refund key.
pub fn validate_wallet_conditions(
    node: &cashu::PublicKey,
    conditions: &cashu::Conditions,
    margin: u64,
    now: u64,
) -> Result<(u64, cashu::PublicKey), LockError> {
    let locktime = conditions.locktime.ok_or(LockError::LockTime)?;
    let pubkeys = conditions.pubkeys.as_ref().ok_or(LockError::Pubkeys)?;
    let refund_keys = conditions
        .refund_keys
        .as_ref()
        .ok_or(LockError::RefundKey)?;
    if conditions.num_sigs != Some(pubkeys.len() as u64) {
        return Err(LockError::NumSigs);
    }
    if refund_keys.len() != 1 {
        return Err(LockError::RefundKey);
    }
    if !pubkeys.contains(node) {
        return Err(LockError::Pubkeys);
    }
    if locktime < now || locktime - now < margin * pubkeys.len() as u64 {
        return Err(LockError::LockTime);
    }
    Ok((locktime, refund_keys[0]))
}

/// Checks an [`exchange_htlc`] that only `recipient` can spend and only `issuer` can refund,
/// leaving `recipient` at least `margin` seconds. Returns its locktime.
pub fn validate_exchange_conditions(
    recipient: &cashu::PublicKey,
    issuer: &cashu::PublicKey,
    conditions: &cashu::Conditions,
    margin: u64,
    now: u64,
) -> Result<u64, LockError> {
    let locktime = conditions.locktime.ok_or(LockError::LockTime)?;
    if conditions.num_sigs != Some(1) {
        return Err(LockError::NumSigs);
    }
    if conditions.pubkeys.as_deref() != Some(std::slice::from_ref(recipient)) {
        return Err(LockError::Pubkeys);
    }
    if conditions.refund_keys.as_deref() != Some(std::slice::from_ref(issuer)) {
        return Err(LockError::RefundKey);
    }
    if locktime < now + margin {
        return Err(LockError::LockTime);
    }
    Ok(locktime)
}

/// The hash lock and validated conditions every proof shares.
pub fn common_lock<T: PartialEq>(
    proofs: &[cashu::Proof],
    validate: impl Fn(&cashu::Conditions) -> Result<T, LockError>,
) -> Result<(Sha256, T), LockError> {
    let mut common = None;
    for proof in proofs {
        let (hash_lock, conditions) = htlc_lock(proof).map_err(|_| LockError::MissingConditions)?;
        let lock = (hash_lock, validate(&conditions)?);
        match &common {
            None => common = Some(lock),
            Some(common) if *common != lock => return Err(LockError::Inconsistent),
            Some(_) => {}
        }
    }
    common.ok_or(LockError::NoProofs)
}

/// Unlocks an HTLC proof with its preimage and the recipient's signature.
pub fn sign_htlc(
    proof: &mut cashu::Proof,
    preimage: &str,
    key: &cashu::SecretKey,
) -> Result<(), cashu::nut01::Error> {
    let signature = key.sign(&proof.secret.to_bytes())?;
    proof.witness = Some(cashu::Witness::HTLCWitness(cashu::HTLCWitness {
        preimage: preimage.to_string(),
        signatures: Some(vec![signature.to_string()]),
    }));
    Ok(())
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
        sign_htlc(&mut proof, preimage, &wallet).expect("sign");
        verify_offline_exchange_htlc(&proof).expect("valid offline htlc");
        assert_eq!(
            htlc_lock(&proof).expect("lock").0,
            offline_hash_lock(preimage)
        );
    }

    fn key() -> cashu::PublicKey {
        cashu::SecretKey::generate().public_key()
    }

    fn conditions_of(spending: cashu::SpendingConditions) -> cashu::Conditions {
        htlc_lock(&proof_with(secret_of(spending)))
            .expect("htlc lock")
            .1
    }

    #[test]
    fn exchange_conditions_admit_only_recipient_and_issuer() {
        let (recipient, issuer, stranger) = (key(), key(), key());
        let (now, margin) = (cashu::util::unix_time(), 600);
        let hash = offline_hash_lock("preimage");
        let locktime = now + margin;
        let ok = conditions_of(exchange_htlc(hash, locktime, recipient, issuer).expect("htlc"));
        assert_eq!(
            validate_exchange_conditions(&recipient, &issuer, &ok, margin, now),
            Ok(locktime)
        );
        assert_eq!(
            validate_exchange_conditions(&stranger, &issuer, &ok, margin, now),
            Err(LockError::Pubkeys)
        );
        assert_eq!(
            validate_exchange_conditions(&recipient, &stranger, &ok, margin, now),
            Err(LockError::RefundKey)
        );
        assert_eq!(
            validate_exchange_conditions(&recipient, &issuer, &ok, margin + 1, now),
            Err(LockError::LockTime)
        );
        let shared = cashu::Conditions::new(
            Some(locktime),
            Some(vec![recipient, stranger]),
            Some(vec![issuer]),
            Some(1),
            None,
            Some(1),
        )
        .expect("conditions");
        assert_eq!(
            validate_exchange_conditions(&recipient, &issuer, &shared, margin, now),
            Err(LockError::Pubkeys),
            "a second pubkey could spend it too"
        );
        let unrefundable = cashu::Conditions::new(
            Some(locktime),
            Some(vec![recipient]),
            None,
            Some(1),
            None,
            None,
        )
        .expect("conditions");
        assert_eq!(
            validate_exchange_conditions(&recipient, &issuer, &unrefundable, margin, now),
            Err(LockError::RefundKey),
            "without the issuer's refund key anyone can spend it after the locktime"
        );
        let refunds = cashu::Conditions::new(
            Some(locktime),
            Some(vec![recipient]),
            Some(vec![issuer, stranger]),
            Some(1),
            None,
            Some(1),
        )
        .expect("conditions");
        assert_eq!(
            validate_exchange_conditions(&recipient, &issuer, &refunds, margin, now),
            Err(LockError::RefundKey),
            "only the issuer may reclaim"
        );
    }

    #[test]
    fn wallet_conditions_leave_every_hop_a_margin() {
        let (node, other, wallet) = (key(), key(), key());
        let (now, margin) = (cashu::util::unix_time(), 600);
        let hash = offline_hash_lock("preimage");
        let locktime = now + 2 * margin;
        let path = vec![other, node];
        let ok = conditions_of(wallet_htlc(hash, locktime, path, wallet).expect("htlc"));
        assert_eq!(
            validate_wallet_conditions(&node, &ok, margin, now),
            Ok((locktime, wallet))
        );
        assert_eq!(
            validate_wallet_conditions(&wallet, &ok, margin, now),
            Err(LockError::Pubkeys)
        );
        assert_eq!(
            validate_wallet_conditions(&node, &ok, margin + 1, now),
            Err(LockError::LockTime)
        );
        assert_eq!(
            validate_wallet_conditions(&node, &ok, 0, locktime + 1),
            Err(LockError::LockTime)
        );
        let one_of_two = cashu::Conditions::new(
            Some(locktime),
            Some(vec![other, node]),
            Some(vec![wallet]),
            Some(1),
            None,
            Some(1),
        )
        .expect("conditions");
        assert_eq!(
            validate_wallet_conditions(&node, &one_of_two, margin, now),
            Err(LockError::NumSigs),
            "every mint on the path must sign"
        );
    }

    #[test]
    fn common_lock_requires_one_hash_lock_and_conditions() {
        let (recipient, issuer) = (key(), key());
        let locktime = cashu::util::unix_time() + 3600;
        let proof = |preimage: &str, locktime: u64| {
            proof_with(secret_of(
                exchange_htlc(offline_hash_lock(preimage), locktime, recipient, issuer)
                    .expect("htlc"),
            ))
        };
        let locktime_of = |c: &cashu::Conditions| c.locktime.ok_or(LockError::LockTime);
        let same = [proof("a", locktime), proof("a", locktime)];
        assert_eq!(
            common_lock(&same, locktime_of),
            Ok((offline_hash_lock("a"), locktime))
        );
        let hashes = [proof("a", locktime), proof("b", locktime)];
        assert_eq!(
            common_lock(&hashes, locktime_of),
            Err(LockError::Inconsistent)
        );
        let locktimes = [proof("a", locktime), proof("a", locktime + 1)];
        assert_eq!(
            common_lock(&locktimes, locktime_of),
            Err(LockError::Inconsistent)
        );
        assert_eq!(common_lock(&[], locktime_of), Err(LockError::NoProofs));
        let plain = proof_with(cashu::secret::Secret::new("plain"));
        assert_eq!(
            common_lock(&[plain], locktime_of),
            Err(LockError::MissingConditions)
        );
    }
}
