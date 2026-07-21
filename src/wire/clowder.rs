// ----- standard library imports
use std::collections::BTreeMap;
// ----- extra library imports
use bitcoin::{XOnlyPublicKey, hashes::sha256::Hash as Sha256Hash, secp256k1};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::{
    core::BillId,
    wire::{attestation::AttestedFingerprints, bill as wire_bill, keys as wire_keys},
};

// ----- end imports

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PathRequest {
    #[schema(value_type = String)]
    pub origin_mint_url: reqwest::Url,
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
    pub mint: reqwest::Url,
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
    /// Earliest beta-reported offline onset, Unix seconds; `Some` iff `alpha_state != Online`.
    pub offline_since: Option<u64>,
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
    /// FROST aggregated public key
    #[schema(value_type = String)]
    pub multisig_agg_xonly: XOnlyPublicKey,
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyEbillMintPaymentRequest {
    #[schema(value_type = String)]
    pub bill_id: BillId,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeriveEbillPaymentAddressRequest {
    #[schema(value_type = String)]
    pub alpha_node_id: bitcoin::secp256k1::PublicKey,
    #[schema(value_type = String)]
    pub bill_id: BillId,
    pub block_id: u64,
    #[schema(value_type = String)]
    pub previous_block_hash: Sha256Hash,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeriveEbillPaymentAddressResponse {
    #[schema(value_type = String)]
    pub payment_address: bitcoin::Address<bitcoin::address::NetworkUnchecked>,
}

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

///--------------------------- Register Ebill

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterEbillRequest {
    pub bill_id: crate::core::BillId,
    pub amount: cashu::Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterEbillResponse {}

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
    pub fingerprints: Vec<wire_keys::ProofFingerprint>,
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
    pub fees: Vec<cashu::BlindSignature>,
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeltOnchainResponse {
    pub txid: bitcoin::Txid,
}

///--------------------------- Melt Quote Onchain

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeltQuoteOnchainRequest {
    pub quote_id: uuid::Uuid,
    pub inputs: AttestedFingerprints,
    pub address: bitcoin::Address<bitcoin::address::NetworkUnchecked>,
    pub admin_fees: cashu::Amount,
    pub network_fees: bitcoin::Amount,
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
    pub inputs: AttestedFingerprints,
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
    pub fees: Vec<cashu::BlindSignature>,
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
    pub proofs: Vec<cashu::Proof>,
    pub signatures: Vec<cashu::BlindSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntermintOriginResponse {
    pub node_id: secp256k1::PublicKey,
    pub mint_url: reqwest::Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofsRequest {
    pub proofs: Vec<cashu::Proof>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintRequest {
    pub proofs: Vec<wire_keys::ProofFingerprint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofsResponse {
    pub proofs: Vec<cashu::Proof>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntermintValidProofs {
    pub valid_proofs: Vec<cashu::Proof>,
    pub amount: cashu::Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidFingerprints {
    pub valid_proofs: Vec<wire_keys::ProofFingerprint>,
    pub amount: cashu::Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckStateRequest {
    pub ys: Vec<cashu::PublicKey>,
    pub ids: Vec<cashu::Id>,
}

///--------------------------- Reply envelope

#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
pub enum ClowderRejection {
    #[error("proof at index {index} already spent")]
    AlreadySpent { index: u32 },
    #[error("commitment inputs reserved")]
    InputsReserved,
    #[error("commitment outputs reserved")]
    OutputsReserved,
    #[error("commitment not found")]
    CommitmentNotFound,
    #[error("commitment mismatch")]
    CommitmentMismatch,
    #[error("signature at index {index} already issued")]
    DuplicateSignature { index: u32 },
    #[error("expired")]
    Expired,
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClowderReply<T> {
    Ok(T),
    Err(ClowderRejection),
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1 as secp;

    fn cbor_roundtrip<T: Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
        let mut bytes = Vec::new();
        ciborium::into_writer(value, &mut bytes).expect("serialize");
        ciborium::from_reader(bytes.as_slice()).expect("deserialize")
    }

    #[test]
    fn clowder_reply_ok_roundtrip() {
        let keypair = secp::Keypair::new_global(&mut rand::thread_rng());
        let msg = secp::Message::from_digest([9u8; 32]);
        let commitment = secp::global::SECP256K1.sign_schnorr(&msg, &keypair);
        let reply = ClowderReply::Ok(SwapCommitmentResponse { commitment });
        match cbor_roundtrip(&reply) {
            ClowderReply::Ok(r) => assert_eq!(r.commitment, commitment),
            ClowderReply::Err(e) => panic!("expected Ok, got {e}"),
        }
    }

    #[test]
    fn clowder_reply_err_roundtrip() {
        let reply = ClowderReply::<SwapCommitmentResponse>::Err(ClowderRejection::AlreadySpent {
            index: 3,
        });
        match cbor_roundtrip(&reply) {
            ClowderReply::Err(ClowderRejection::AlreadySpent { index }) => assert_eq!(index, 3),
            other => panic!("expected AlreadySpent, got {other:?}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysetRequest {
    pub keyset: cashu::KeySet,
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
    pub amount: cashu::Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintUrlRequest {
    pub mint_url: reqwest::Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintUrlResponse {
    pub mint_url: reqwest::Url,
}

///--------------------------- Generic Onchain information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnchainFeesEstimateRequest {
    /// the target amount to send onchain
    pub target: bitcoin::Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnchainFeesEstimateResponse {
    pub fees: bitcoin::Amount,
}
