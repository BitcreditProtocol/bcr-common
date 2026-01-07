use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintOnchainRequest {
    pub mint_signature: String,
    pub keyset_id: cashu::Id,
    pub quote_id: uuid::Uuid,
    pub amount: cashu::Amount,
    pub expiry: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintOnchainResponse {
    pub signatures: Vec<cashu::BlindSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintEiouRequest {
    pub mint_signature: String,
    pub keyset_id: cashu::Id,
    pub quote_id: uuid::Uuid,
    pub amount: cashu::Amount,
    pub expiry: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintEiouResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeltOnchainRequest {
    pub quote: uuid::Uuid,
    pub address: bitcoin::Address<bitcoin::address::NetworkUnchecked>,
    pub amount: bitcoin::Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeltOnchainResponse {
    pub txid: bitcoin::Txid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRequest {
    pub proofs: Vec<cashu::Proof>,
    pub blinds: Vec<cashu::BlindedMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResponse {
    pub signatures: Vec<cashu::BlindSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HearbeatRequest {
    pub timestamp: u64,
}
