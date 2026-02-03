// ----- standard library imports
// ----- extra library imports
use bitcoin::{hashes::sha256::Hash as Sha256Hash, secp256k1};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::wire::{bill as wire_bill, keys as wire_keys};
pub mod messages;
// ----- end imports

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PathRequest {
    #[schema(value_type = String)]
    pub origin_mint_url: cashu::MintUrl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyResponse {
    pub public_key: secp256k1::PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

///--------------------------- Alpha State
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum SimpleAlphaState {
    /// Last seen timestamp
    Online(u64),
    /// Last seen timestamp
    Interim(u64),
    /// Last seen timestamp
    Offline(u64),
    /// Pre Rabid
    Rabid(String),
    /// Post Rabid
    ConfiscatedRabid(bitcoin::Txid, bitcoin::secp256k1::PublicKey, String),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlphaStateResponse {
    pub state: SimpleAlphaState,
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
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

///--------------------------- Accounting

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SupplyResponse {
    pub credit: cashu::Amount,
    pub debit: cashu::Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BitcoinAmountResponse {
    #[schema(value_type = u64)]
    pub amount: bitcoin::Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EbillAmountResponse {
    #[schema(value_type = u64)]
    pub amount: bitcoin::Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EiouAmountResponse {
    pub amount: u64,
}

/// Collateral backing eCash and circulating supply information regarding eCash
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Coverage {
    pub debit_circulating_supply: cashu::Amount,
    pub credit_circulating_supply: cashu::Amount,
    #[schema(value_type = u64)]
    pub onchain_collateral: bitcoin::Amount,
    #[schema(value_type = u64)]
    pub ebill_collateral: bitcoin::Amount,
    pub eiou_collateral: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintCollateralResponse {
    #[schema(value_type = u64)]
    pub onchain: bitcoin::Amount,
    #[schema(value_type = u64)]
    pub ebill: bitcoin::Amount,
    pub eiou: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintCirculatingSupplyResponse {
    pub debit: cashu::Amount,
    pub credit: cashu::Amount,
}

///--------------------------- Clowder Node Information

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClowderNodeInfo {
    #[schema(value_type = String)]
    pub change_address: bitcoin::Address<bitcoin::address::NetworkUnchecked>,
    pub node_id: cashu::PublicKey,
    pub uptime_timestamp: u64,
    pub version: String,
    #[schema(value_type = String)]
    pub network: bitcoin::Network,
}

///--------------------------- Onchain Mint Information

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnchainAddressRequest {
    #[schema(value_type = String)]
    pub quote_id: uuid::Uuid,
    pub keyset_id: cashu::Id,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnchainAddressResponse {
    #[schema(value_type = String)]
    pub address: bitcoin::Address<bitcoin::address::NetworkUnchecked>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyMintPaymentResponse {
    #[schema(value_type = u64)]
    pub amount: bitcoin::Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyMintPaymentRequest {
    #[schema(value_type = String)]
    pub quote_id: uuid::Uuid,
    pub keyset_id: cashu::Id,
    pub min_confirmations: u32,
}
