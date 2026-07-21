// ----- standard library imports
// ----- extra library imports
use bitcoin::hashes::{Hash, sha256::Hash as Sha256};
use frost_secp256k1_tr::{SigningPackage, round1::SigningCommitments, round2::SignatureShare};
use serde::{Deserialize, Serialize};
// ----- local imports
use crate::wire::clowder as wire_clowder;

// ----- end imports

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MintStream {
    Swap(wire_clowder::SwapRequest, wire_clowder::SwapResponse),
    MintOnchain(
        wire_clowder::MintOnchainRequest,
        wire_clowder::MintOnchainResponse,
    ),
    MintEiou(
        wire_clowder::MintEiouRequest,
        wire_clowder::MintEiouResponse,
    ),
    MintEbill(
        wire_clowder::MintEbillRequest,
        wire_clowder::MintEbillResponse,
    ),
    RegisterEbill(
        wire_clowder::RegisterEbillRequest,
        wire_clowder::RegisterEbillResponse,
    ),
    MintForeignEcash(
        wire_clowder::MintForeignEcashRequest,
        wire_clowder::MintForeignEcashResponse,
    ),
    MintForeignOfflineEcash(
        wire_clowder::MintForeignOfflineEcashRequest,
        wire_clowder::MintForeignOfflineEcashResponse,
    ),
    MeltOnchain(wire_clowder::MeltOnchainRequest),
    MeltQuoteOnchain(wire_clowder::MeltQuoteOnchainRequest),
    MintQuoteOnchain(wire_clowder::MintQuoteOnchainRequest),
    OfflineExchangeSign(wire_clowder::OfflineExchangeSignRequest),
    SwapCommitment(wire_clowder::SwapCommitmentRequest),
    CreateKeyset(
        wire_clowder::KeysetCreationRequest,
        wire_clowder::KeysetCreationResponse,
    ),
    BillRequestToPay(
        wire_clowder::RequestToPayEbillRequest,
        wire_clowder::RequestToPayEbillResponse,
    ),
    Heartbeat(
        wire_clowder::HeartbeatRequest,
        wire_clowder::HeartbeatResponse,
    ),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedSwapRequest {
    pub inputs: Vec<cashu::Proof>,
    pub outputs: Vec<cashu::BlindedMessage>,
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
    pub pubkey: bitcoin::secp256k1::PublicKey,
    pub signature: bitcoin::secp256k1::schnorr::Signature,
}
impl SignedSwapRequest {
    pub(crate) fn msg_to_sign(
        inputs: &[cashu::Proof],
        outputs: &[cashu::BlindedMessage],
    ) -> Sha256 {
        const LOWER_HEX_PUBKEY_SIZE: usize = 66;
        // Pre-calculate capacity to avoid reallocations
        let capacity =
            inputs.len() * LOWER_HEX_PUBKEY_SIZE + (outputs.len() * LOWER_HEX_PUBKEY_SIZE);
        let mut msg = Vec::with_capacity(capacity);
        for input in inputs.iter() {
            msg.extend_from_slice(input.c.to_hex().as_bytes());
        }
        for output in outputs.iter() {
            msg.extend_from_slice(output.blinded_secret.to_hex().as_bytes());
        }
        Sha256::hash(&msg)
    }

    pub fn sign(
        inputs: Vec<cashu::Proof>,
        outputs: Vec<cashu::BlindedMessage>,
        secret_key: cashu::SecretKey,
    ) -> Option<bitcoin::secp256k1::schnorr::Signature> {
        let msg = Self::msg_to_sign(&inputs, &outputs);
        secret_key.sign(msg.as_ref()).ok()
    }

    pub fn verify(&self) -> bool {
        let msg = Self::msg_to_sign(&self.inputs, &self.outputs);
        let msg = bitcoin::secp256k1::Message::from_digest_slice(msg.as_ref())
            .expect("Digest from Sha 256");
        self.signature
            .verify(&msg, &self.pubkey.x_only_public_key().0)
            .is_ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostRequest {
    /// Clowder aggregated group key
    pub aggregated_key: bitcoin::secp256k1::PublicKey,
    /// PSBT being signed; lets the signatory re-derive each input sighash
    #[serde(with = "psbt_as_bytes")]
    pub psbt: bitcoin::psbt::Psbt,
    /// Collection of commitments from involved signers
    pub signing_packages: Vec<SigningPackage>,
    /// Tweaks for each signer to apply
    pub tweaks: Vec<Option<[u8; 32]>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostResponse {
    /// Partial signature of each involved signer
    pub signature_shares: Vec<SignatureShare>,
    /// New refreshed commitments
    pub commitments: Vec<TimedCommitment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentsRequest {
    /// Request fresh commitments
    pub aggregated_key: bitcoin::secp256k1::PublicKey,
    /// Amount to request
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimedCommitment {
    pub commitment: SigningCommitments,
    pub created_at_epoch_secs: u64,
    pub expires_at_epoch_secs: u64,
}

impl TimedCommitment {
    pub fn new(commitment: SigningCommitments, ttl_seconds: u64, now: u64) -> Self {
        Self {
            commitment,
            created_at_epoch_secs: now,
            expires_at_epoch_secs: now + ttl_seconds,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentsResponse {
    /// Returned commitments
    pub commitments: Vec<TimedCommitment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyPackageRequest {
    /// Request the public key package for a particular multisig
    pub aggregated_key: bitcoin::secp256k1::PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyPackageResponse {
    /// Public Key Package
    pub package_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysetConfigRequest {
    pub aggregated_key: bitcoin::secp256k1::PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysetConfigResponse {
    pub min_signers: u16,
}

pub mod psbt_as_bytes {
    use bitcoin::psbt::Psbt;
    use serde::{Deserializer, Serializer, de};

    pub fn serialize<S: Serializer>(psbt: &Psbt, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&psbt.serialize())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Psbt, D::Error> {
        let bytes: Vec<u8> = de::Deserialize::deserialize(d)?;
        Psbt::deserialize(&bytes).map_err(de::Error::custom)
    }
}
