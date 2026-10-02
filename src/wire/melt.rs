// ----- standard library imports
// ----- extra library imports
use bitcoin::{Amount, address::NetworkUnchecked};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports
use crate::wire::{
    attestation::AttestedFingerprints,
    borsh::{
        deserialize_btc_amount, deserialize_from_str, deserialize_unchecked_address,
        serialize_as_str, serialize_btc_amount, serialize_unchecked_address,
    },
    common::ProtestStatus,
};
// ----- end imports

///--------------------------- Melt Quote Onchain Request
#[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct MeltQuoteOnchainRequest {
    pub inputs: AttestedFingerprints,
    /// Bitcoin address the wallet wants the mint to pay
    #[borsh(
        serialize_with = "serialize_unchecked_address",
        deserialize_with = "deserialize_unchecked_address"
    )]
    pub address: bitcoin::Address<NetworkUnchecked>,
    /// the amount the user wants the mint to pay to the address
    #[borsh(
        serialize_with = "serialize_btc_amount",
        deserialize_with = "deserialize_btc_amount"
    )]
    pub amount: Amount,
    /// total tx fee in sats the user pays for the onchain transaction
    #[borsh(
        serialize_with = "serialize_btc_amount",
        deserialize_with = "deserialize_btc_amount"
    )]
    pub network_fee: Amount,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub wallet_key: cashu::PublicKey,
}

///--------------------------- Melt Quote Onchain Response Body
#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct MeltQuoteOnchainResponseBody {
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub quote: uuid::Uuid,
    pub inputs: AttestedFingerprints,
    #[borsh(
        serialize_with = "serialize_unchecked_address",
        deserialize_with = "deserialize_unchecked_address"
    )]
    pub address: bitcoin::Address<NetworkUnchecked>,
    /// the amount the mint will pay for the proofs in the quote
    #[borsh(
        serialize_with = "serialize_btc_amount",
        deserialize_with = "deserialize_btc_amount"
    )]
    pub amount: Amount,
    /// total tx fee in sats the user pays for the onchain transaction
    #[borsh(
        serialize_with = "serialize_btc_amount",
        deserialize_with = "deserialize_btc_amount"
    )]
    pub network_fee: Amount,
    /// the melt fee in sats charged by the mint
    #[borsh(
        serialize_with = "serialize_btc_amount",
        deserialize_with = "deserialize_btc_amount"
    )]
    pub melt_fee: Amount,
    /// Unix timestamp when the commitment expires
    pub expiry: u64,
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub wallet_key: cashu::PublicKey,
}

///--------------------------- Melt Quote Onchain Response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltQuoteOnchainResponse {
    pub content: String,
    #[schema(value_type = String)]
    pub commitment: bitcoin::secp256k1::schnorr::Signature,
}

///--------------------------- Melt Onchain Request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltOnchainRequest {
    #[schema(value_type = String)]
    pub quote: uuid::Uuid,
    pub inputs: Vec<cashu::Proof>,
}

///--------------------------- Melt Onchain Response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltOnchainResponse {
    #[schema(value_type = String)]
    pub txid: bitcoin::Txid,
}

///--------------------------- Melt Protest Request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltProtestRequest {
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

///--------------------------- Melt Protest Response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltProtestResponse {
    pub status: ProtestStatus,
    #[schema(value_type = Option<String>)]
    pub txid: Option<bitcoin::Txid>,
}

///--------------------------- Melt Onchain Estimate Request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltOnchainEstimateRequest {
    /// the amount the user wants the mint to pay to the address
    #[schema(value_type = u64)]
    pub amount: Amount,
    /// when set, the recipient output is sized from its script type
    #[schema(value_type = Option<String>)]
    #[serde(default)]
    pub address: Option<bitcoin::Address<NetworkUnchecked>>,
}

///--------------------------- Fee Rate Estimate
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct FeeRateEstimate {
    pub target_blocks: u16,
    pub sat_per_vb: f32,
}

///--------------------------- Melt Onchain Estimate Response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltOnchainEstimateResponse {
    /// estimated tx virtual size in vbytes for paying the requested amount
    pub tx_vsize: u64,
    /// current fee rates per confirmation target
    pub feerates: Vec<FeeRateEstimate>,
    /// the melt fee in sats the mint charges for the requested amount
    #[schema(value_type = u64)]
    pub melt_fee: Amount,
    /// the melt fee rate in parts per thousand
    pub melt_fee_ppk: u64,
}

///--------------------------- Melt Onchain Config Response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MeltOnchainConfigResponse {
    /// the melt fee rate in parts per thousand
    pub melt_fee_ppk: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::attestation::IssuanceAttestation;
    use bitcoin::secp256k1 as secp;

    fn sample_attested_fingerprints() -> AttestedFingerprints {
        let keypair = secp::Keypair::new_global(&mut rand::thread_rng());
        let msg = secp::Message::from_digest([7u8; 32]);
        let signature = secp::global::SECP256K1.sign_schnorr(&msg, &keypair);
        AttestedFingerprints {
            inputs: vec![],
            attestation: IssuanceAttestation {
                beta_id: keypair.public_key(),
                fp_digest: [1u8; 32],
                coords_mac: [2u8; 32],
                signature,
            },
        }
    }

    fn sample_address() -> bitcoin::Address<NetworkUnchecked> {
        "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080"
            .parse()
            .expect("valid address")
    }

    // ----- golden vectors -------------------------------------------------------
    //
    // `MeltQuoteOnchainResponseBody`'s borsh bytes are signed: `client::mint::
    // onchain_melt_quote` returns `(content, commitment)` where `content` is this
    // encoding and `commitment` is a signature over it, so changing these bytes, or
    // those of the `AttestedFingerprints` inside them, breaks something outside this
    // crate. `MeltQuoteOnchainRequest` itself goes over HTTP as JSON (`client::mint`
    // posts through `jsonrpc::Client`) and the NATS path sends the separate
    // `wire::clowder::MeltQuoteOnchainRequest` as CBOR, so its borsh bytes are on no
    // wire today; its vector only keeps the derive stable for whoever signs it next.
    //
    // The round-trip tests below cannot catch that: they pass whenever a
    // `serialize_with`/`deserialize_with` pair changes together, which is exactly what
    // editing those attributes does.
    //
    // The fixtures use literal key material rather than the `sample_*` helpers above,
    // for two reasons: those generate a random keypair, and `sign_schnorr` mixes in
    // auxiliary randomness, so even a fixed key would not give fixed bytes. Nothing
    // verifies the signature while serializing -- it has to be stable, not valid.
    //
    // If one of these fails after a dependency bump rather than a local edit, that is
    // not a flaky test: `Cargo.lock` is not committed and these bytes include `Display`
    // output from `cashu` and `bitcoin` types, so a change there is a real wire change
    // arriving through a dependency.

    /// secp256k1 generator point, compressed -- recognisable rather than arbitrary.
    const FIXED_PUBKEY: &str = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
    /// 64 bytes. Stable, and deliberately not a valid signature over anything.
    const FIXED_SIGNATURE: &str = concat!(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );

    const WIRE_CHANGED: &str = "the wire encoding changed. These bytes are signed and sent \
                                between services, so signatures from other builds will no \
                                longer verify and peers on another version will disagree. If \
                                the change is deliberate, update the constant and treat it as \
                                a breaking wire change.";

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// One input with every field set, `dleq` included, so the vectors cover the
    /// `ProofFingerprint` encoding and its custom serializers, not just a `Vec` length.
    fn fixed_fingerprint() -> crate::wire::keys::ProofFingerprint {
        crate::wire::keys::ProofFingerprint {
            keyset_id: "00ad268c4d1f5826".parse().expect("valid keyset id"),
            amount: 8,
            y: fixed_wallet_key(),
            c: fixed_wallet_key(),
            dleq: Some(cashu::ProofDleq::new(
                "11".repeat(32).parse().expect("valid secret key"),
                "22".repeat(32).parse().expect("valid secret key"),
                "33".repeat(32).parse().expect("valid secret key"),
            )),
        }
    }

    fn fixed_attested_fingerprints() -> AttestedFingerprints {
        AttestedFingerprints {
            inputs: vec![fixed_fingerprint()],
            attestation: IssuanceAttestation {
                beta_id: FIXED_PUBKEY.parse().expect("valid public key"),
                fp_digest: [1u8; 32],
                coords_mac: [2u8; 32],
                signature: FIXED_SIGNATURE.parse().expect("64-byte signature"),
            },
        }
    }

    fn fixed_wallet_key() -> cashu::PublicKey {
        FIXED_PUBKEY.parse().expect("valid cashu key")
    }

    /// A one-element `Vec`: the fingerprint's keyset id, `y` and `c` as borsh
    /// `String`s around its `u64` amount, then `dleq` as `Some` of three hex `String`s.
    /// Then the attestation: `beta_id` and `signature` as borsh `String`s around two raw
    /// `[u8; 32]` arrays, which borsh writes with no length prefix of their own.
    const ATTESTED_FINGERPRINTS_BORSH: &str = concat!(
        "0100000010000000303061643236386334643166353832360800000000000000",
        "4200000030323739626536363765663964636262616335356130363239356365",
        "3837306230373032396266636462326463653238643935396632383135623136",
        "6638313739384200000030323739626536363765663964636262616335356130",
        "3632393563653837306230373032396266636462326463653238643935396632",
        "3831356231366638313739380140000000313131313131313131313131313131",
        "3131313131313131313131313131313131313131313131313131313131313131",
        "3131313131313131313131313131313131400000003232323232323232323232",
        "3232323232323232323232323232323232323232323232323232323232323232",
        "3232323232323232323232323232323232323232324000000033333333333333",
        "3333333333333333333333333333333333333333333333333333333333333333",
        "3333333333333333333333333333333333333333333333333342000000303237",
        "3962653636376566396463626261633535613036323935636538373062303730",
        "3239626663646232646365323864393539663238313562313666383137393801",
        "0101010101010101010101010101010101010101010101010101010101010102",
        "0202020202020202020202020202020202020202020202020202020202020280",
        "0000006161616161616161616161616161616161616161616161616161616161",
        "6161616161616161616161616161616161616161616161616161616161616161",
        "6161616262626262626262626262626262626262626262626262626262626262",
        "6262626262626262626262626262626262626262626262626262626262626262",
        "626262",
    );

    /// `inputs`, the address as a borsh `String` -- `serialize_unchecked_address`
    /// writes `assume_checked().to_string()`, so the network is not in the bytes --
    /// then two amounts as little-endian `u64` satoshis, then `wallet_key`.
    const MELT_QUOTE_ONCHAIN_REQUEST_BORSH: &str = concat!(
        "0100000010000000303061643236386334643166353832360800000000000000",
        "4200000030323739626536363765663964636262616335356130363239356365",
        "3837306230373032396266636462326463653238643935396632383135623136",
        "6638313739384200000030323739626536363765663964636262616335356130",
        "3632393563653837306230373032396266636462326463653238643935396632",
        "3831356231366638313739380140000000313131313131313131313131313131",
        "3131313131313131313131313131313131313131313131313131313131313131",
        "3131313131313131313131313131313131400000003232323232323232323232",
        "3232323232323232323232323232323232323232323232323232323232323232",
        "3232323232323232323232323232323232323232324000000033333333333333",
        "3333333333333333333333333333333333333333333333333333333333333333",
        "3333333333333333333333333333333333333333333333333342000000303237",
        "3962653636376566396463626261633535613036323935636538373062303730",
        "3239626663646232646365323864393539663238313562313666383137393801",
        "0101010101010101010101010101010101010101010101010101010101010102",
        "0202020202020202020202020202020202020202020202020202020202020280",
        "0000006161616161616161616161616161616161616161616161616161616161",
        "6161616161616161616161616161616161616161616161616161616161616161",
        "6161616262626262626262626262626262626262626262626262626262626262",
        "6262626262626262626262626262626262626262626262626262626262626262",
        "6262622c00000062637274317177353038643671656a7874646734793572337a",
        "6172766172793063357877376b796774303830d007000000000000fa00000000",
        "0000004200000030323739626536363765663964636262616335356130363239",
        "3563653837306230373032396266636462326463653238643935396632383135",
        "623136663831373938",
    );

    /// As the request, plus `quote` first and `melt_fee` and `expiry` before
    /// `wallet_key`. `expiry` is a plain `u64`, with no custom serializer.
    const MELT_QUOTE_ONCHAIN_RESPONSE_BODY_BORSH: &str = concat!(
        "2400000030303030303030302d303030302d303030302d303030302d30303030",
        "3030303030303261010000001000000030306164323638633464316635383236",
        "0800000000000000420000003032373962653636376566396463626261633535",
        "6130363239356365383730623037303239626663646232646365323864393539",
        "6632383135623136663831373938420000003032373962653636376566396463",
        "6262616335356130363239356365383730623037303239626663646232646365",
        "3238643935396632383135623136663831373938014000000031313131313131",
        "3131313131313131313131313131313131313131313131313131313131313131",
        "3131313131313131313131313131313131313131313131313140000000323232",
        "3232323232323232323232323232323232323232323232323232323232323232",
        "3232323232323232323232323232323232323232323232323232323232400000",
        "0033333333333333333333333333333333333333333333333333333333333333",
        "3333333333333333333333333333333333333333333333333333333333333333",
        "3342000000303237396265363637656639646362626163353561303632393563",
        "6538373062303730323962666364623264636532386439353966323831356231",
        "3666383137393801010101010101010101010101010101010101010101010101",
        "0101010101010102020202020202020202020202020202020202020202020202",
        "0202020202020280000000616161616161616161616161616161616161616161",
        "6161616161616161616161616161616161616161616161616161616161616161",
        "6161616161616161616161626262626262626262626262626262626262626262",
        "6262626262626262626262626262626262626262626262626262626262626262",
        "62626262626262626262622c00000062637274317177353038643671656a7874",
        "646734793572337a6172766172793063357877376b796774303830d007000000",
        "000000fa00000000000000140000000000000000f15365000000004200000030",
        "3237396265363637656639646362626163353561303632393563653837306230",
        "3730323962666364623264636532386439353966323831356231366638313739",
        "38",
    );

    #[test]
    fn attested_fingerprints_borsh_encoding_is_frozen() {
        let bytes = borsh::to_vec(&fixed_attested_fingerprints()).expect("borsh serialize");
        assert_eq!(hex(&bytes), ATTESTED_FINGERPRINTS_BORSH, "{WIRE_CHANGED}");
    }

    #[test]
    fn melt_quote_onchain_request_borsh_encoding_is_frozen() {
        let request = MeltQuoteOnchainRequest {
            inputs: fixed_attested_fingerprints(),
            address: sample_address(),
            amount: Amount::from_sat(2000),
            network_fee: Amount::from_sat(250),
            wallet_key: fixed_wallet_key(),
        };
        let bytes = borsh::to_vec(&request).expect("borsh serialize");
        assert_eq!(
            hex(&bytes),
            MELT_QUOTE_ONCHAIN_REQUEST_BORSH,
            "{WIRE_CHANGED}"
        );
    }

    #[test]
    fn melt_quote_onchain_response_body_borsh_encoding_is_frozen() {
        let body = MeltQuoteOnchainResponseBody {
            quote: uuid::Uuid::from_u128(42),
            inputs: fixed_attested_fingerprints(),
            address: sample_address(),
            amount: Amount::from_sat(2000),
            network_fee: Amount::from_sat(250),
            melt_fee: Amount::from_sat(20),
            expiry: 1_700_000_000,
            wallet_key: fixed_wallet_key(),
        };
        let bytes = borsh::to_vec(&body).expect("borsh serialize");
        assert_eq!(
            hex(&bytes),
            MELT_QUOTE_ONCHAIN_RESPONSE_BODY_BORSH,
            "{WIRE_CHANGED}"
        );
    }

    fn sample_wallet_key() -> cashu::PublicKey {
        let keypair = secp::Keypair::new_global(&mut rand::thread_rng());
        keypair
            .public_key()
            .to_string()
            .parse()
            .expect("valid cashu key")
    }

    #[test]
    fn melt_quote_onchain_request_borsh_roundtrip() {
        let request = MeltQuoteOnchainRequest {
            inputs: sample_attested_fingerprints(),
            address: sample_address(),
            amount: Amount::from_sat(2000),
            network_fee: Amount::from_sat(250),
            wallet_key: sample_wallet_key(),
        };
        let bytes = borsh::to_vec(&request).expect("borsh serialize");
        let back: MeltQuoteOnchainRequest = borsh::from_slice(&bytes).expect("borsh deserialize");
        assert_eq!(back.inputs, request.inputs);
        assert_eq!(back.address, request.address);
        assert_eq!(back.amount, request.amount);
        assert_eq!(back.network_fee, request.network_fee);
        assert_eq!(back.wallet_key, request.wallet_key);
    }

    #[test]
    fn melt_quote_onchain_response_body_borsh_roundtrip() {
        let body = MeltQuoteOnchainResponseBody {
            quote: uuid::Uuid::from_u128(42),
            inputs: sample_attested_fingerprints(),
            address: sample_address(),
            amount: Amount::from_sat(2000),
            network_fee: Amount::from_sat(250),
            melt_fee: Amount::from_sat(20),
            expiry: 1_700_000_000,
            wallet_key: sample_wallet_key(),
        };
        let bytes = borsh::to_vec(&body).expect("borsh serialize");
        let back: MeltQuoteOnchainResponseBody =
            borsh::from_slice(&bytes).expect("borsh deserialize");
        assert_eq!(back.quote, body.quote);
        assert_eq!(back.inputs, body.inputs);
        assert_eq!(back.address, body.address);
        assert_eq!(back.amount, body.amount);
        assert_eq!(back.network_fee, body.network_fee);
        assert_eq!(back.melt_fee, body.melt_fee);
        assert_eq!(back.expiry, body.expiry);
        assert_eq!(back.wallet_key, body.wallet_key);
    }

    #[test]
    fn melt_onchain_estimate_json_roundtrip() {
        let request = MeltOnchainEstimateRequest {
            amount: Amount::from_sat(2000),
            address: Some(sample_address()),
        };
        let json = serde_json::to_string(&request).expect("serialize");
        let back: MeltOnchainEstimateRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.amount, request.amount);
        assert_eq!(back.address, request.address);
        let back: MeltOnchainEstimateRequest =
            serde_json::from_str(r#"{"amount":2000}"#).expect("deserialize");
        assert_eq!(back.address, None);

        let response = MeltOnchainEstimateResponse {
            tx_vsize: 154,
            feerates: vec![
                FeeRateEstimate {
                    target_blocks: 1,
                    sat_per_vb: 5.5,
                },
                FeeRateEstimate {
                    target_blocks: 6,
                    sat_per_vb: 2.0,
                },
            ],
            melt_fee: Amount::from_sat(20),
            melt_fee_ppk: 10,
        };
        let json = serde_json::to_string(&response).expect("serialize");
        let back: MeltOnchainEstimateResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.tx_vsize, response.tx_vsize);
        assert_eq!(back.feerates, response.feerates);
        assert_eq!(back.melt_fee, response.melt_fee);
        assert_eq!(back.melt_fee_ppk, response.melt_fee_ppk);
    }

    #[test]
    fn melt_onchain_config_json_roundtrip() {
        let response = MeltOnchainConfigResponse { melt_fee_ppk: 10 };
        let json = serde_json::to_string(&response).expect("serialize");
        let back: MeltOnchainConfigResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.melt_fee_ppk, response.melt_fee_ppk);
    }
}
