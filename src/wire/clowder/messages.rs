use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::wire::keys::ProofFingerprint;

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
pub struct BillPaymentRequest {
    pub bill_id: crate::core::BillId,
    pub payment_clowder_quote: uuid::Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillPaymentResponse {}

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
    pub proofs: Vec<cashu::Proof>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeltOnchainResponse {
    pub txid: crate::wire::melt::MeltTx,
}

///--------------------------- Swap Commitment

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapCommitmentRequest {
    pub content: String,
    pub wallet_key: cashu::PublicKey,
    pub wallet_signature: bitcoin::secp256k1::schnorr::Signature,
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
