// ----- standard library imports
// ----- extra library imports
pub use crate::cashu::{
    Amount, BlindSignature, BlindedMessage, CurrencyUnit, Id, KeySet, MeltOptions, MeltRequest,
    MintUrl, Proof, ProofDleq, PublicKey as CashuPublicKey, SecretKey, SpendingConditions, Witness,
    dhke::hash_to_curve,
};
use crate::wire::clowder::messages;
use crate::wire::keys::ProofFingerprint;
use bitcoin::secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
// ----- end imports

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintSwapRequest {
    pub proofs: Vec<Proof>,
    pub signatures: Vec<BlindSignature>,
}

/// For testing, remove
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintEiouRequest {
    pub quote_id: uuid::Uuid,
    pub keyset_id: Id,
    pub amount: Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintOnchainRequest {
    pub quote_id: uuid::Uuid,
    /// Let mint sign from private key corresponding to payment utxo
    pub mint_signature: String,
    pub signatures: Vec<BlindSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntermintOriginResponse {
    pub node_id: PublicKey,
    pub mint_url: MintUrl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofsRequest {
    pub proofs: Vec<Proof>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintRequest {
    pub proofs: Vec<ProofFingerprint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofsResponse {
    pub proofs: Vec<Proof>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntermintValidProofs {
    pub valid_proofs: Vec<Proof>,
    pub amount: Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidFingerprints {
    pub valid_proofs: Vec<ProofFingerprint>,
    pub amount: Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckStateRequest {
    pub ys: Vec<CashuPublicKey>,
    pub ids: Vec<Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysetRequest {
    pub keyset: KeySet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MintStream {
    Swap(messages::SwapRequest, messages::SwapResponse),
    MintOnchain(messages::MintOnchainRequest, messages::MintOnchainResponse),
    MintEiou(messages::MintEiouRequest, messages::MintEiouResponse),
    MintEbill(messages::MintEbillRequest, messages::MintEbillResponse),
    MintForeignEcash(
        messages::MintForeignEcashRequest,
        messages::MintForeignEcashResponse,
    ),
    MintForeignOfflineEcash(
        messages::MintForeignOfflineEcashRequest,
        messages::MintForeignOfflineEcashResponse,
    ),
    MeltOnchain(messages::MeltOnchainRequest),
    MeltQuoteOnchain(messages::MeltQuoteOnchainRequest),
    MintQuoteOnchain(messages::MintQuoteOnchainRequest),
    OfflineExchangeSign(messages::OfflineExchangeSignRequest),
    SwapCommitment(messages::SwapCommitmentRequest),
    CreateKeyset(
        messages::KeysetCreationRequest,
        messages::KeysetCreationResponse,
    ),
    BillRequestToPay(
        messages::RequestToPayEbillRequest,
        messages::RequestToPayEbillResponse,
    ),
    Heartbeat(messages::HeartbeatRequest, messages::HeartbeatResponse),
    DeactivateKeyset(messages::KeysetDeactivationRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastOfflineResponse {
    pub timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmountResponse {
    pub amount: Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintUrlRequest {
    pub mint_url: MintUrl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintUrlResponse {
    pub mint_url: MintUrl,
}
