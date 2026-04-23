// ----- standard library imports
use std::collections::BTreeMap;
// ----- extra library imports
use bitcoin::secp256k1::PublicKey;
use cashu::{Amount, BlindSignature, Id, KeySet, Proof, PublicKey as CashuPublicKey};
use serde::{Deserialize, Serialize};
// ----- local imports
use crate::wire::keys::ProofFingerprint;

// ----- end imports

///--------------------------- Keyset Creation

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysetCreationRequest {
    pub id: cashu::Id,
    pub expiry: u64,
    pub unit: cashu::CurrencyUnit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysetCreationResponse {
    pub public_keys: BTreeMap<cashu::Amount, cashu::PublicKey>,
    pub id: cashu::Id,
    pub expiry: u64,
    pub unit: cashu::CurrencyUnit,
}

///--------------------------- Keyset Deactivation

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysetDeactivationRequest {
    pub id: cashu::Id,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysetDeactivationResponse {
    pub id: cashu::Id,
}

///--------------------------- Mint Onchain

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintOnchainRequest {
    pub keyset_id: cashu::Id,
    pub quote_id: uuid::Uuid,
    pub amount: cashu::Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintOnchainResponse {
    pub signatures: Vec<cashu::BlindSignature>,
}

///--------------------------- Redemption

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestToPayEbillRequest {
    pub payment_address: bitcoin::Address<bitcoin::address::NetworkUnchecked>,
    pub bill_id: crate::core::BillId,
    pub block_id: u64,
    pub previous_block_hash: bitcoin::hashes::sha256::Hash,
    pub amount: bitcoin::Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestToPayEbillResponse {}

///--------------------------- Mint Ebill

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintEbillRequest {
    pub keyset_id: cashu::Id,
    pub quote_id: uuid::Uuid,
    pub bill_id: crate::core::BillId,
    pub amount: cashu::Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintEbillResponse {
    pub signatures: Vec<cashu::BlindSignature>,
}

///--------------------------- Mint Foreign eCash

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintForeignEcashRequest {
    pub proofs: Vec<cashu::Proof>,
    pub exchange_path: Vec<bitcoin::secp256k1::PublicKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintForeignEcashResponse {
    pub proofs: Vec<cashu::Proof>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintForeignOfflineEcashRequest {
    pub fingerprints: Vec<ProofFingerprint>,
    pub hashes: Vec<bitcoin::hashes::sha256::Hash>,
    pub wallet_pk: cashu::PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintForeignOfflineEcashResponse {
    pub proofs: Vec<cashu::Proof>,
}

///--------------------------- Mint EIOU

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintEiouRequest {
    pub keyset_id: cashu::Id,
    pub quote_id: uuid::Uuid,
    pub amount: cashu::Amount,
    pub expiry: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintEiouResponse {}

///--------------------------- Melt Onchain

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeltOnchainRequest {
    pub quote: uuid::Uuid,
    pub address: bitcoin::Address<bitcoin::address::NetworkUnchecked>,
    pub amount: bitcoin::Amount,
    pub inputs: Vec<cashu::Proof>,
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeltOnchainResponse {
    pub txid: crate::wire::melt::MeltTx,
}

///--------------------------- Melt Quote Onchain

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeltQuoteOnchainRequest {
    pub quote_id: uuid::Uuid,
    pub inputs: Vec<ProofFingerprint>,
    pub address: bitcoin::Address<bitcoin::address::NetworkUnchecked>,
    pub amount: bitcoin::Amount,
    pub total: cashu::Amount,
    pub expiry: u64,
    pub wallet_key: cashu::PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeltQuoteOnchainResponse {
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

///--------------------------- Mint Quote Onchain

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintQuoteOnchainRequest {
    pub quote_id: uuid::Uuid,
    pub address: String,
    pub payment_amount: bitcoin::Amount,
    pub expiry: u64,
    pub blinded_messages: Vec<cashu::nuts::BlindedMessage>,
    pub wallet_key: cashu::PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintQuoteOnchainResponse {
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

///--------------------------- Offline Exchange Sign

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineExchangeSignRequest {
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineExchangeSignResponse {
    pub signature: bitcoin::secp256k1::schnorr::Signature,
}

///--------------------------- Swap Commitment

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapCommitmentRequest {
    pub inputs: Vec<ProofFingerprint>,
    pub outputs: Vec<cashu::BlindedMessage>,
    pub expiry: u64,
    pub wallet_key: cashu::PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapCommitmentResponse {
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

///--------------------------- Swap

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRequest {
    pub proofs: Vec<cashu::Proof>,
    pub blinds: Vec<cashu::BlindedMessage>,
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResponse {
    pub signatures: Vec<cashu::BlindSignature>,
}

///--------------------------- Heartbeat

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintSwapRequest {
    pub proofs: Vec<Proof>,
    pub signatures: Vec<BlindSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntermintOriginResponse {
    pub node_id: PublicKey,
    pub mint_url: reqwest::Url,
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
    pub mint_url: reqwest::Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintUrlResponse {
    pub mint_url: reqwest::Url,
}
