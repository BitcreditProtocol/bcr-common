// ----- standard library imports
// ----- extra library imports
use bitcoin::Amount;
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
pub struct MintRequest {
    /// Quote ID
    #[schema(value_type = String)]
    pub quote: uuid::Uuid,
    /// Blinded messages to be signed
    pub outputs: Vec<cashu::BlindedMessage>,
    /// Signature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// Onchain Mint quote response with commitment signature
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnchainMintQuoteResponse {
    pub content: String, // base64, borsh serialized OnchainMintQuoteResponseBody
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

/// Mint response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintResponse {
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
        let request = MintRequest {
            quote: uuid::Uuid::from_u128(rand::random()),
            outputs: blinds,
            signature: Some(String::from("signature")),
        };
        let bytes = serde_json::to_vec(&request).expect("serialize");
        let deserialized: cashu::MintRequest<uuid::Uuid> =
            serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized.quote, request.quote);
        assert_eq!(
            deserialized.outputs[0].blinded_secret,
            request.outputs[0].blinded_secret
        );
        assert_eq!(deserialized.signature, request.signature);
    }

    #[test]
    fn mintresponse_json_wire_compat() {
        let (kinfo, _) = core_tests::generate_random_ecash_keyset();
        let pk = cashu::PublicKey::from(core::generate_random_keypair().public_key());
        let response = MintResponse {
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
