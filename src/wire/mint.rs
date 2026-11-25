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
use crate::wire::{
    borsh::{
        deserialize_btc_amount, deserialize_from_str, deserialize_vec_of_jsons,
        deserialize_vecof_blindedmessage, serialize_as_str, serialize_btc_amount,
        serialize_vec_of_jsons, serialize_vecof_blindedmessage,
    },
    common::{ProtestOutcome, ProtestOutcomeError, ProtestStatus, secp_compat},
};
// ----- end imports

/// Onchain Mint quote request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnchainMintQuoteRequest {
    /// Blinded messages to be signed upon payment, keyset must be SAT
    pub blinded_messages: Vec<cashu::BlindedMessage>,
    #[schema(value_type = String)]
    #[serde(with = "secp_compat")]
    pub wallet_key: secp::PublicKey,
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
    #[borsh(
        serialize_with = "serialize_btc_amount",
        deserialize_with = "deserialize_btc_amount"
    )]
    #[schema(value_type = u64)]
    pub payment_amount: Amount,
    /// Quote expiry timestamp
    pub expiry: u64,
    /// Blinded messages committed to
    #[borsh(
        serialize_with = "serialize_vecof_blindedmessage",
        deserialize_with = "deserialize_vecof_blindedmessage"
    )]
    pub blinded_messages: Vec<cashu::nuts::BlindedMessage>,
    #[schema(value_type = String)]
    #[serde(with = "secp_compat")]
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub wallet_key: secp::PublicKey,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, BorshSerialize, BorshDeserialize)]
pub struct OnchainMintQuoteResponseBodyV1 {
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
    #[borsh(
        serialize_with = "serialize_btc_amount",
        deserialize_with = "deserialize_btc_amount"
    )]
    #[schema(value_type = u64)]
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
    #[serde(with = "secp_compat")]
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub wallet_key: secp::PublicKey,
}

/// Onchain Mint Request to Fetch Signatures
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnchainMintRequest {
    /// Quote ID
    #[schema(value_type = String)]
    pub quote: uuid::Uuid,
    /// Id of the origin mint
    #[schema(value_type = String)]
    pub alpha_id: secp::PublicKey,
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
    pub signature: secp::schnorr::Signature,
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
    pub commitment: secp::schnorr::Signature,
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
    pub alpha_id: secp::PublicKey,
    #[schema(value_type = String)]
    pub quote_id: uuid::Uuid,
    pub content: String,
    #[schema(value_type = String)]
    pub commitment: secp::schnorr::Signature,
    #[schema(value_type = String)]
    pub wallet_signature: secp::schnorr::Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MintProtestResponse {
    pub status: ProtestStatus,
    pub signatures: Option<Vec<cashu::nuts::BlindSignature>>,
}

impl TryFrom<MintProtestResponse> for ProtestOutcome<Vec<cashu::nuts::BlindSignature>> {
    type Error = ProtestOutcomeError;
    fn try_from(response: MintProtestResponse) -> Result<Self, Self::Error> {
        let signatures = response.signatures.filter(|s| !s.is_empty());
        Self::from_parts(response.status, signatures)
    }
}

impl From<ProtestOutcome<Vec<cashu::nuts::BlindSignature>>> for MintProtestResponse {
    fn from(outcome: ProtestOutcome<Vec<cashu::nuts::BlindSignature>>) -> Self {
        let (status, signatures) = outcome.into_parts();
        Self { status, signatures }
    }
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
        let blinds = core_tests::generate_random_ecash_blindedmessages(kinfo.id.into(), &amounts)
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
                keyset_id: kinfo.id.into(),
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
                keyset_id: kinfo.id.into(),
                c: pk,
                dleq: None,
            }],
        };
        let bytes = serde_json::to_vec(&response).expect("serialize");
        let deserialized: cashu::MintResponse =
            serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(deserialized.signatures[0].c, response.signatures[0].c);
    }

    /// The onchain mint-quote messages as they were laid out before `wallet_key`
    /// became `secp::PublicKey`.
    mod wire_compat {
        use super::*;
        use crate::{ecash, wire::common::assert_wire_compat};

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct OnchainMintQuoteRequestV1 {
            blinded_messages: Vec<cashu::BlindedMessage>,
            wallet_key: cashu::PublicKey,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
        struct OnchainMintQuoteResponseBodyV1Keyed {
            #[borsh(
                serialize_with = "serialize_as_str",
                deserialize_with = "deserialize_from_str"
            )]
            quote: uuid::Uuid,
            address: String,
            #[borsh(
                serialize_with = "serialize_btc_amount",
                deserialize_with = "deserialize_btc_amount"
            )]
            payment_amount: Amount,
            expiry: u64,
            #[borsh(
                serialize_with = "serialize_vecof_blindedmessage",
                deserialize_with = "deserialize_vecof_blindedmessage"
            )]
            blinded_messages: Vec<cashu::nuts::BlindedMessage>,
            #[borsh(
                serialize_with = "serialize_as_str",
                deserialize_with = "deserialize_from_str"
            )]
            wallet_key: cashu::PublicKey,
        }

        fn sample_body(wallet_key: secp::PublicKey) -> OnchainMintQuoteResponseBody {
            OnchainMintQuoteResponseBody {
                quote: uuid::Uuid::from_u128(11),
                address: "an-address".to_string(),
                payment_amount: Amount::from_sat(1000),
                expiry: 99,
                blinded_messages: vec![],
                wallet_key,
            }
        }

        fn as_v1(body: &OnchainMintQuoteResponseBody) -> OnchainMintQuoteResponseBodyV1Keyed {
            OnchainMintQuoteResponseBodyV1Keyed {
                quote: body.quote,
                address: body.address.clone(),
                payment_amount: body.payment_amount,
                expiry: body.expiry,
                blinded_messages: body.blinded_messages.clone(),
                wallet_key: ecash::public_key_secp2cashu(&body.wallet_key),
            }
        }

        #[test]
        fn onchain_mint_quote_request_wire_compat() {
            let wallet_key = core::generate_random_keypair().public_key();
            let request = OnchainMintQuoteRequest {
                blinded_messages: vec![],
                wallet_key,
            };
            let v1 = OnchainMintQuoteRequestV1 {
                blinded_messages: vec![],
                wallet_key: ecash::public_key_secp2cashu(&wallet_key),
            };
            assert_wire_compat(&request, &v1);
        }

        #[test]
        fn onchain_mint_quote_response_body_wire_compat() {
            let body = sample_body(core::generate_random_keypair().public_key());
            assert_wire_compat(&body, &as_v1(&body));
        }

        /// The response body is also borshed into the signed envelope the mint commits to.
        #[test]
        fn onchain_mint_quote_response_body_borsh_wire_compat() {
            let body = sample_body(core::generate_random_keypair().public_key());
            let v1 = as_v1(&body);
            let bytes = borsh::to_vec(&body).expect("serialize");
            let v1_bytes = borsh::to_vec(&v1).expect("serialize");
            assert_eq!(bytes, v1_bytes, "borsh encodings diverge");
            let as_v1_again: OnchainMintQuoteResponseBodyV1Keyed =
                borsh::from_slice(&bytes).expect("v1 reads the new encoding");
            assert_eq!(as_v1_again, v1);
            let back: OnchainMintQuoteResponseBody =
                borsh::from_slice(&v1_bytes).expect("the new type reads a v1 blob");
            assert_eq!(borsh::to_vec(&back).expect("serialize"), bytes);
        }
    }
}
