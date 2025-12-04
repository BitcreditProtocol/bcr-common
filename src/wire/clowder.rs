// ----- standard library imports
// ----- extra library imports
use bitcoin::{hashes::sha256::Hash as Sha256Hash, secp256k1};
use serde::{Deserialize, Serialize};
// ----- local imports
use crate::wire::keys as wire_keys;

// ----- end imports

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRequest {
    pub origin_mint_url: cashu::MintUrl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyResponse {
    pub public_key: secp256k1::PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineResponse {
    pub offline: bool,
}

///--------------------------- Connected Mint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedMintResponse {
    pub mint: cashu::MintUrl,
    pub clowder: reqwest::Url,
    pub node_id: secp256k1::PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedMintsResponse {
    pub mint_urls: Vec<cashu::MintUrl>,
    pub clowder_urls: Vec<reqwest::Url>,
    pub node_ids: Vec<secp256k1::PublicKey>,
}

///--------------------------- Exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRequest {
    pub alpha_proofs: Vec<cashu::Proof>,
    pub exchange_path: Vec<secp256k1::PublicKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeResponse {
    pub beta_proofs: Vec<cashu::Proof>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstituteExchangeRequest {
    pub proofs: Vec<wire_keys::ProofFingerprint>,
    pub locks: Vec<Sha256Hash>,
    pub wallet_pubkey: secp256k1::PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstituteExchangeResponse {
    pub outputs: Vec<cashu::Proof>,
    pub signature: secp256k1::schnorr::Signature,
}

///--------------------------- Rabid Reason
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq)]
pub enum RabidReason {
    Forked(u64, Sha256Hash, Sha256Hash),
    HashSeqDiscrepancy(u64, Sha256Hash, Sha256Hash),
    // TODO needs to be signed by a time service so the timestamp can't be made up
    Offline(u64),
}
// Hash order doesn't matter
impl PartialEq for RabidReason {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RabidReason::Forked(a1, a2, a3), RabidReason::Forked(b1, b2, b3)) => {
                a1 == b1 && ((a2, a3) == (b2, b3) || (a2, a3) == (b3, b2))
            }
            (
                RabidReason::HashSeqDiscrepancy(a1, a2, a3),
                RabidReason::HashSeqDiscrepancy(b1, b2, b3),
            ) => a1 == b1 && ((a2, a3) == (b2, b3) || (a2, a3) == (b3, b2)),
            (RabidReason::Offline(a), RabidReason::Offline(b)) => a == b,
            _ => false,
        }
    }
}

///--------------------------- Alpha State
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AlphaState {
    /// Last seen timestamp
    Online(u64),
    /// Last seen timestamp
    Offline(u64),
    /// Pre Rabid
    Rabid(RabidReason),
    /// Post Rabid
    ConfiscatedRabid(bitcoin::Txid, RabidReason),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlphaStateResponse {
    pub state: AlphaState,
}
