// ----- standard library imports
// ----- extra library imports
use bitcoin::{hashes::sha256::Hash as Sha256Hash, secp256k1};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::wire::{bill as wire_bill, keys as wire_keys};

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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConnectedMintResponse {
    #[schema(value_type = String)]
    pub mint: cashu::MintUrl,
    pub clowder: reqwest::Url,
    #[schema(value_type = String)]
    pub node_id: secp256k1::PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConnectedMintsResponse {
    pub mints: Vec<ConnectedMintResponse>,
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, ToSchema)]
pub enum RabidReason {
    Forked(u64, Sha256Hash, Sha256Hash),
    HashSeqDiscrepancy(u64, Sha256Hash, Sha256Hash),
    // TODO needs to be signed by a time service so the timestamp can't be made up
    Offline(u64),
    #[schema(value_type = String)]
    InvalidBurn(bitcoin::secp256k1::PublicKey),
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlphaStateResponse {
    pub state: AlphaState,
}

///--------------------------- Wallet-side Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalletEvent {
    Swap {
        minted: Vec<cashu::BlindSignature>,
    },
    Mint {
        minted: Vec<cashu::BlindSignature>,
    },
    Melt {
        burned: Vec<cashu::PublicKey>,
        qid: String,
    },
}

///--------------------------- Redemption activation Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedemptionActivationEvent {
    pub keyset_id: cashu::KeySetInfo,
    pub ebills: Vec<wire_bill::BillShortDescription>,
}

///--------------------------- Perceived State
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum MintState {
    Online,
    Offline,
    Interim,
    Rabid,
}
/// Reflects what the majority of Beta mints think about the current Alpha mint
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PerceivedState {
    #[schema(value_type = Option<String>)]
    pub substitute_beta: Option<bitcoin::secp256k1::PublicKey>,
    pub alpha_state: MintState,
}
