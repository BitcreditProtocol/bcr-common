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
