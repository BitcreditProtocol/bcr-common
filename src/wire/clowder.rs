// ----- standard library imports
use std::collections::BTreeMap;
// ----- extra library imports
use bitcoin::{
    XOnlyPublicKey,
    hashes::{Hash, sha256::Hash as Sha256Hash},
    secp256k1,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::{
    core::BillId,
    wire::{
        attestation::AttestedFingerprints, bill as wire_bill, exchange as wire_exchange,
        keys as wire_keys,
    },
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
    /// Frozen-tip digest the wallet signs exchanges against. `Some` iff `offline`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_digest: Option<[u8; 32]>,
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
    /// Address dedicated to receiving brc-20 eIOU tokens
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub eiou_address: Option<bitcoin::Address<bitcoin::address::NetworkUnchecked>>,
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
    /// Required on offline issuances. Optional (with `wallet_signature`) only so
    /// older ledger entries re-serialize byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange_digest: Option<[u8; 32]>,
    /// Over the exchange digest; only wallet-signed reports can spend its proofs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wallet_signature: Option<secp256k1::schnorr::Signature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintForeignOfflineEcashResponse {
    pub proofs: Vec<cashu::Proof>,
}

///--------------------------- Offline Spend (Alpha's recovery ledger entries)
/// One exchange marked spent on Alpha's chain. Carries the wallet's signed
/// claim verbatim so any Beta can authenticate the spend on its own.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineSpendRequest {
    pub exchange: wire_exchange::ExchangeBroadcast,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineSpendResponse {}

/// Streamed after the spend entries. A Beta resumes lease acks once this
/// marker is on the chain and every exchange it witnessed is spent on it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutageCloseRequest {
    pub evidence_digest: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutageCloseResponse {}

///--------------------------- Offline Redeem (Alpha issues against a spend entry)
/// Carries the wallet's claim verbatim, so any Beta can judge the issuance from the
/// spend entry already on the chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineRedeemRequest {
    pub redemption: wire_exchange::RedeemOfflineExchangeRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineRedeemResponse {
    pub signatures: Vec<cashu::BlindSignature>,
}

/// What the node authorised; the mint issues against this, never the request's total.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RedeemOfflineExchangeAuthorization {
    pub amount: cashu::Amount,
}

///--------------------------- Lease Acknowledgment

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseAckRequest {
    pub alpha_id: secp256k1::PublicKey,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseAckResponse {
    pub beta_id: secp256k1::PublicKey,
    pub alpha_id: secp256k1::PublicKey,
    pub tip_seq: u64,
    /// Echoes the request timestamp; signed so an old ack cannot renew a lease.
    pub timestamp: u64,
    pub signature: secp256k1::schnorr::Signature,
}

/// Domain separation tag for a Beta lease acknowledgment signature.
pub const DOMAIN_TAG_LEASE: &[u8] = b"bcr/exchange/lease/v1";

/// `SHA256(DOMAIN_TAG_LEASE || alpha_id || beta_id || tip_seq || timestamp)`.
pub fn lease_ack_message(
    alpha_id: &secp256k1::PublicKey,
    beta_id: &secp256k1::PublicKey,
    tip_seq: u64,
    timestamp: u64,
) -> secp256k1::Message {
    let mut msg = Vec::with_capacity(DOMAIN_TAG_LEASE.len() + 33 + 33 + 8 + 8);
    msg.extend_from_slice(DOMAIN_TAG_LEASE);
    msg.extend_from_slice(&alpha_id.serialize());
    msg.extend_from_slice(&beta_id.serialize());
    msg.extend_from_slice(&tip_seq.to_be_bytes());
    msg.extend_from_slice(&timestamp.to_be_bytes());
    secp256k1::Message::from_digest(*Sha256Hash::hash(&msg).as_ref())
}

impl LeaseAckResponse {
    /// True only for a verified ack to this exact poll: same alpha and timestamp.
    pub fn authenticates(&self, request: &LeaseAckRequest) -> bool {
        self.alpha_id == request.alpha_id
            && self.timestamp == request.timestamp
            && self.verify().is_ok()
    }

    pub fn verify(&self) -> Result<(), secp256k1::Error> {
        let msg = lease_ack_message(&self.alpha_id, &self.beta_id, self.tip_seq, self.timestamp);
        secp256k1::global::SECP256K1.verify_schnorr(
            &self.signature,
            &msg,
            &self.beta_id.x_only_public_key().0,
        )
    }
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
    /// total tx fee the user pays; None = legacy dynamic-fee path
    /// optional to keep ledger entries byte-identical under CBOR re-serialization
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_fee: Option<bitcoin::Amount>,
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
    #[error("invalid fees")]
    InvalidFees,
    #[error("mint lease expired")]
    LeaseExpired,
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

    /// Ledger entries are hashed and signature-verified over their CBOR bytes,
    /// and historical entries are re-serialized and re-verified. A request
    /// without `network_fee` must keep its legacy byte encoding.
    #[test]
    fn melt_onchain_request_legacy_cbor_compat() {
        #[derive(Serialize, Deserialize)]
        struct LegacyMeltOnchainRequest {
            quote: uuid::Uuid,
            address: bitcoin::Address<bitcoin::address::NetworkUnchecked>,
            amount: bitcoin::Amount,
            inputs: Vec<cashu::Proof>,
            fees: Vec<cashu::BlindSignature>,
            commitment: bitcoin::secp256k1::schnorr::Signature,
        }

        let keypair = secp::Keypair::new_global(&mut rand::thread_rng());
        let msg = secp::Message::from_digest([7u8; 32]);
        let commitment = secp::global::SECP256K1.sign_schnorr(&msg, &keypair);
        let quote = uuid::Uuid::from_u128(42);
        let address: bitcoin::Address<bitcoin::address::NetworkUnchecked> =
            "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080"
                .parse()
                .expect("valid address");
        let amount = bitcoin::Amount::from_sat(2000);

        let legacy = LegacyMeltOnchainRequest {
            quote,
            address: address.clone(),
            amount,
            inputs: vec![],
            fees: vec![],
            commitment,
        };
        let current = MeltOnchainRequest {
            quote,
            address,
            amount,
            inputs: vec![],
            fees: vec![],
            commitment,
            network_fee: None,
        };

        let mut legacy_bytes = Vec::new();
        ciborium::into_writer(&legacy, &mut legacy_bytes).expect("serialize legacy");
        let mut current_bytes = Vec::new();
        ciborium::into_writer(&current, &mut current_bytes).expect("serialize current");
        assert_eq!(
            legacy_bytes, current_bytes,
            "None must re-serialize byte-identical to the legacy encoding"
        );

        let decoded: MeltOnchainRequest =
            ciborium::from_reader(legacy_bytes.as_slice()).expect("decode legacy blob");
        assert_eq!(decoded.network_fee, None);

        let with_fee = MeltOnchainRequest {
            network_fee: Some(bitcoin::Amount::from_sat(250)),
            ..current
        };
        let back = cbor_roundtrip(&with_fee);
        assert_eq!(back.network_fee, Some(bitcoin::Amount::from_sat(250)));
    }

    #[test]
    fn mint_foreign_offline_ecash_request_legacy_cbor_compat() {
        #[derive(Serialize, Deserialize)]
        struct LegacyMintForeignOfflineEcashRequest {
            fingerprints: Vec<wire_keys::ProofFingerprint>,
            hashes: Vec<bitcoin::hashes::sha256::Hash>,
            wallet_pk: cashu::PublicKey,
        }

        let keypair = secp::Keypair::new_global(&mut rand::thread_rng());
        let wallet_pk: cashu::PublicKey = keypair.public_key().into();
        let hashes = vec![bitcoin::hashes::Hash::hash(&[1u8])];

        let legacy = LegacyMintForeignOfflineEcashRequest {
            fingerprints: vec![],
            hashes: hashes.clone(),
            wallet_pk,
        };
        let current = MintForeignOfflineEcashRequest {
            fingerprints: vec![],
            hashes,
            wallet_pk,
            exchange_digest: None,
            wallet_signature: None,
        };

        let mut legacy_bytes = Vec::new();
        ciborium::into_writer(&legacy, &mut legacy_bytes).expect("serialize legacy");
        let mut current_bytes = Vec::new();
        ciborium::into_writer(&current, &mut current_bytes).expect("serialize current");
        assert_eq!(
            legacy_bytes, current_bytes,
            "None must re-serialize byte-identical to the legacy encoding"
        );

        let decoded: MintForeignOfflineEcashRequest =
            ciborium::from_reader(legacy_bytes.as_slice()).expect("decode legacy blob");
        assert_eq!(decoded.exchange_digest, None);
        assert_eq!(decoded.wallet_signature, None);

        let msg = secp::Message::from_digest([5u8; 32]);
        let with_digest = MintForeignOfflineEcashRequest {
            exchange_digest: Some([5u8; 32]),
            wallet_signature: Some(secp::global::SECP256K1.sign_schnorr(&msg, &keypair)),
            ..current
        };
        let back = cbor_roundtrip(&with_digest);
        assert_eq!(back.exchange_digest, Some([5u8; 32]));
        assert_eq!(back.wallet_signature, with_digest.wallet_signature);
    }

    #[test]
    fn offline_spend_request_cbor_roundtrip() {
        let wallet = secp::Keypair::new_global(&mut rand::thread_rng());
        let req = OfflineSpendRequest {
            exchange: wire_exchange::tests_support::sample_broadcast(&wallet),
        };
        let back = cbor_roundtrip(&req);
        assert_eq!(back.exchange.exchange_digest, req.exchange.exchange_digest);
        assert_eq!(back.exchange.wallet_pk, req.exchange.wallet_pk);

        // A Beta that never saw the broadcast can still authenticate the spend.
        back.exchange.verify().expect("spend carries its own proof");

        let mut forged = req.clone();
        forged.exchange.fingerprints = wire_exchange::tests_support::sample_fingerprints(&[8]);
        assert!(
            forged.exchange.verify().is_err(),
            "naming other proofs needs a wallet signature nobody has"
        );
    }

    #[test]
    fn offline_redeem_request_cbor_roundtrip() {
        let wallet = secp::Keypair::new_global(&mut rand::thread_rng());
        let alpha = secp::Keypair::new_global(&mut rand::thread_rng()).public_key();
        let outputs = wire_exchange::tests_support::sample_outputs(&[1, 2]);
        let req = OfflineRedeemRequest {
            redemption: wire_exchange::RedeemOfflineExchangeRequest::new(
                &alpha, [7u8; 32], outputs, &wallet,
            ),
        };
        let back = cbor_roundtrip(&req);
        assert_eq!(
            back.redemption.exchange_digest,
            req.redemption.exchange_digest
        );

        // A Beta reading the entry off the chain can judge the issuance itself.
        let amount = cashu::Amount::from(3_u64);
        back.redemption
            .verify(&alpha, &wallet.public_key().into(), amount)
            .expect("entry carries its own proof");

        let mut forged = req.clone();
        forged.redemption.outputs = wire_exchange::tests_support::sample_outputs(&[1, 2]);
        assert!(
            forged
                .redemption
                .verify(&alpha, &wallet.public_key().into(), amount)
                .is_err(),
            "naming other outputs needs a wallet signature nobody has"
        );
    }

    #[test]
    fn lease_ack_response_sign_verify() {
        let alpha = secp::Keypair::new_global(&mut rand::thread_rng());
        let beta = secp::Keypair::new_global(&mut rand::thread_rng());
        let msg = lease_ack_message(&alpha.public_key(), &beta.public_key(), 42, 100);
        let signature = secp::global::SECP256K1.sign_schnorr(&msg, &beta);
        let ack = LeaseAckResponse {
            beta_id: beta.public_key(),
            alpha_id: alpha.public_key(),
            tip_seq: 42,
            timestamp: 100,
            signature,
        };
        ack.verify().unwrap();

        let request = LeaseAckRequest {
            alpha_id: alpha.public_key(),
            timestamp: 100,
        };
        assert!(ack.authenticates(&request));

        let ack_for_another_poll = LeaseAckRequest {
            timestamp: 101,
            ..request
        };
        assert!(!ack.authenticates(&ack_for_another_poll));

        let stale = LeaseAckResponse {
            timestamp: 101,
            ..ack
        };
        assert!(stale.verify().is_err());
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnchainTxEstimateRequest {
    /// the target amount to send onchain
    pub amount: bitcoin::Amount,
    /// when set, the recipient output is sized from its script type
    #[serde(default)]
    pub address: Option<bitcoin::Address<bitcoin::address::NetworkUnchecked>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnchainTxEstimateResponse {
    /// estimated tx virtual size in vbytes for sending the requested amount
    pub tx_vsize: u64,
    /// current fee rates per confirmation target
    pub feerates: Vec<crate::wire::melt::FeeRateEstimate>,
}

///--------------------------- Reserve funding
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct AddReserveRequest {
    #[schema(value_type = String)]
    pub reserve_id: uuid::Uuid,
    #[schema(value_type = u64)]
    pub amount: bitcoin::Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct AddReserveResponse {
    #[schema(value_type = String)]
    pub reserve_id: uuid::Uuid,
    #[schema(value_type = String)]
    pub address: bitcoin::Address<bitcoin::address::NetworkUnchecked>,
    #[schema(value_type = u64)]
    pub amount: bitcoin::Amount,
    pub status: AddReserveStatus,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq, strum::EnumDiscriminants,
)]
#[strum_discriminants(
    derive(strum::Display, strum::EnumString),
    strum(serialize_all = "lowercase")
)]
pub enum AddReserveStatus {
    Pending,
    Completed {
        #[schema(value_type = String)]
        outpoint: bitcoin::OutPoint,
    },
    FundingMismatch,
}
