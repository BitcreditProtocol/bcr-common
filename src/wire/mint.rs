// ----- standard library imports
// ----- extra library imports
use bitcoin::{
    Amount,
    hashes::{Hash, sha256::Hash as Sha256Hash},
    secp256k1 as secp,
};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::wire::borsh::{
    deserialize_btc_amount, deserialize_from_str, deserialize_vec_of_jsons, serialize_as_str,
    serialize_btc_amount, serialize_vec_of_jsons,
};
// ----- end imports

/// Onchain Mint quote request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnchainMintQuoteRequest {
    /// Blinded messages to be signed upon payment, keyset must be SAT
    pub blinded_messages: Vec<cashu::BlindedMessage>,
    #[schema(value_type = String)]
    pub wallet_key: cashu::PublicKey,
}

/// Onchain Mint quote response body
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, BorshSerialize, BorshDeserialize)]
pub struct OnchainMintQuoteResponseBody {
    /// Quote ID
    #[schema(value_type = String)]
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub quote: uuid::Uuid,
    /// Bitcoin address to send payment
    pub address: String,
    /// Amount to pay including fees
    #[schema(value_type = u64)]
    #[borsh(
        serialize_with = "serialize_btc_amount",
        deserialize_with = "deserialize_btc_amount"
    )]
    pub payment_amount: Amount,
    /// Quote expiry timestamp
    pub expiry: u64,
    /// Blinded messages committed to
    #[borsh(
        serialize_with = "serialize_vec_of_jsons",
        deserialize_with = "deserialize_vec_of_jsons"
    )]
    pub blinded_messages: Vec<cashu::nuts::BlindedMessage>,
    #[schema(value_type = String)]
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub wallet_key: cashu::PublicKey,
}

/// Onchain Mint Request to Fetch Signatures
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnchainMintRequest {
    /// Quote ID
    #[schema(value_type = String)]
    pub quote: uuid::Uuid,
    /// Id of the origin mint
    #[schema(value_type = String)]
    pub alpha_id: bitcoin::secp256k1::PublicKey,
}

/// Mint request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EbillMintRequest {
    /// Quote ID
    #[schema(value_type = String)]
    pub quote: uuid::Uuid,
    /// Blinded messages to be signed
    pub outputs: Vec<cashu::BlindedMessage>,
    /// Signature
    #[schema(value_type = String)]
    pub signature: bitcoin::secp256k1::schnorr::Signature,
}
impl EbillMintRequest {
    pub fn new(quote: uuid::Uuid, outputs: Vec<cashu::BlindedMessage>, kp: &secp::Keypair) -> Self {
        let msg = Self::msg_to_sign(quote, &outputs);
        let signature = secp::global::SECP256K1.sign_schnorr(&msg, kp);
        Self {
            quote,
            outputs,
            signature,
        }
    }
    fn msg_to_sign(qid: uuid::Uuid, outputs: &[cashu::BlindedMessage]) -> secp::Message {
        let quote_id = qid.to_string();
        let capacity = quote_id.len() + (outputs.len() * 66);
        let mut raw = Vec::with_capacity(capacity);
        raw.append(&mut quote_id.clone().into_bytes()); // String.into_bytes() produces UTF-8
        for output in outputs {
            raw.extend_from_slice(output.blinded_secret.to_hex().as_bytes());
        }
        let hash: Sha256Hash = Sha256Hash::hash(&raw);
        secp::Message::from_digest(*hash.as_ref())
    }

    pub fn verify_signature(&self, pk: &secp::PublicKey) -> bool {
        let msg = Self::msg_to_sign(self.quote, &self.outputs);
        secp::global::SECP256K1
            .verify_schnorr(&self.signature, &msg, &pk.x_only_public_key().0)
            .is_ok()
    }
}

/// Onchain Mint quote response with commitment signature
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnchainMintQuoteResponse {
    pub content: String, // base64, borsh serialized OnchainMintQuoteResponseBody
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

/// Onchain Mint response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnchainMintResponse {
    pub signatures: Vec<cashu::BlindSignature>,
}

/// E-Bill Mint response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EbillMintResponse {
    pub signatures: Vec<cashu::BlindSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintProtestRequest {
    #[schema(value_type = String)]
    pub alpha_id: bitcoin::secp256k1::PublicKey,
    #[schema(value_type = String)]
    pub quote_id: uuid::Uuid,
    pub content: String,
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
    #[schema(value_type = String)]
    pub wallet_signature: bitcoin::secp256k1::schnorr::Signature,
}

pub use crate::wire::common::ProtestStatus;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintProtestResponse {
    pub status: ProtestStatus,
    pub signatures: Option<Vec<cashu::nuts::BlindSignature>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{self, test_utils as core_tests};

    #[test]
    fn mintrequest_json_wire_compat() {
        let (kinfo, _) = core_tests::generate_random_ecash_keyset();
        let amounts = vec![
            cashu::Amount::from(8),
            cashu::Amount::from(16),
            cashu::Amount::from(32),
        ];
        let blinds = core_tests::generate_random_ecash_blindedmessages(kinfo.id, &amounts)
            .into_iter()
            .map(|(b, _, _)| b)
            .collect();
        let kp = core::generate_random_keypair();
        let request = EbillMintRequest::new(uuid::Uuid::new_v4(), blinds, &kp);
        let bytes = serde_json::to_vec(&request).expect("serialize");
        let deserialized: cashu::MintRequest<uuid::Uuid> =
            serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized.quote, request.quote);
        assert_eq!(
            deserialized.outputs[0].blinded_secret,
            request.outputs[0].blinded_secret
        );
        assert_eq!(
            deserialized.signature.as_ref().unwrap(),
            &request.signature.to_string()
        );
        let cpk = cashu::PublicKey::from(kp.public_key());
        deserialized.verify_signature(cpk).unwrap()
    }

    #[test]
    fn ebill_mintresponse_json_wire_compat() {
        let (kinfo, _) = core_tests::generate_random_ecash_keyset();
        let pk = cashu::PublicKey::from(core::generate_random_keypair().public_key());
        let response = EbillMintResponse {
            signatures: vec![cashu::BlindSignature {
                amount: cashu::Amount::from(rand::random::<u16>() as u64),
                keyset_id: kinfo.id,
                c: pk,
                dleq: None,
            }],
        };
        let bytes = serde_json::to_vec(&response).expect("serialize");
        let deserialized: cashu::MintResponse =
            serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized.signatures[0].c, response.signatures[0].c);
    }

    #[test]
    fn onchain_mintresponse_json_wire_compat() {
        let (kinfo, _) = core_tests::generate_random_ecash_keyset();
        let pk = cashu::PublicKey::from(core::generate_random_keypair().public_key());
        let response = OnchainMintResponse {
            signatures: vec![cashu::BlindSignature {
                amount: cashu::Amount::from(rand::random::<u16>() as u64),
                keyset_id: kinfo.id,
                c: pk,
                dleq: None,
            }],
        };
        let bytes = serde_json::to_vec(&response).expect("serialize");
        let deserialized: cashu::MintResponse =
            serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized.signatures[0].c, response.signatures[0].c);
    }
}
