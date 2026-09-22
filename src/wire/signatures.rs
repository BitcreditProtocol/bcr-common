// ----- standard library imports
// ----- extra library imports
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
// ----- local imports
use crate::{
    core::BillId,
    wire::borsh::{
        deserialize_from_str, deserialize_from_u64, deserialize_tstamp, serialize_as_str,
        serialize_as_u64, serialize_tstamp,
    },
};

/// --------------------------- request to mint from ebill description
#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct RequestToMintFromEBillDesc {
    pub ebill_id: BillId,
    #[borsh(
        serialize_with = "serialize_tstamp",
        deserialize_with = "deserialize_tstamp"
    )]
    pub deadline: time::OffsetDateTime,
    pub sweeping_address: String, // bitcoin::Address is either Serialize or Deserialize
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignedRequestToMintFromEBillDesc {
    pub content: String, // base64 borsh serialized RequestToMintFromEBillDesc
    pub signature: bitcoin::secp256k1::schnorr::Signature,
}

/// --------------------------- request to melt
#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct RequestToMeltDesc {
    #[borsh(
        serialize_with = "serialize_as_str",
        deserialize_with = "deserialize_from_str"
    )]
    pub qid: uuid::Uuid,
    #[borsh(
        serialize_with = "serialize_as_u64",
        deserialize_with = "deserialize_from_u64"
    )]
    pub amount: cashu::Amount,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignedRequestToMeltDesc {
    pub content: String, // base64 borsh serialized RequestToMeltDesc
    pub signature: bitcoin::secp256k1::schnorr::Signature,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    use crate::core::BillId;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    // These two constants freeze the borsh encoding of the payloads that get signed.
    //
    // `core::signature::serialize_borsh_msg_b64` signs `Sha256(borsh::to_vec(msg))`, and
    // `SignedRequestToMeltDesc::content` / `SignedRequestToMintFromEBillDesc::content`
    // carry that same encoding base64'd. So a change to these bytes -- reordering fields,
    // or swapping either `serialize_with` -- invalidates every signature produced by
    // another build of this crate, silently and across process boundaries.
    //
    // The existing round-trip tests elsewhere in `wire` cannot see that: they pass
    // whenever the serializer and deserializer change together, which is exactly what
    // happens when someone edits a `#[borsh(...)]` attribute pair.
    //
    // Both values are derived from the borsh format and the field attributes rather than
    // recorded from a run: every field here becomes a borsh `String` -- a four-byte
    // little-endian length followed by UTF-8 -- except `amount`, which `serialize_as_u64`
    // writes as a little-endian `u64`.
    //
    // `deadline` is the one field whose encoding is not the borsh default: `serialize_tstamp`
    // writes `OffsetDateTime::unix_timestamp`, a little-endian `i64`. It is also the field
    // that already moved once. Until #176 landed on 2026-09-09 this payload carried the
    // deadline as a 23-byte string, chrono's `Display` for `DateTime<Utc>`
    // (`2023-11-14 22:13:20 UTC`), so that migration changed what every signature commits
    // to. Nothing in the suite went red, because the serializer and the deserializer moved
    // together. That is the failure these two constants exist to make visible.
    //
    // If one of them fails after a dependency bump rather than a local edit, that is not a
    // false alarm either: `Cargo.lock` is not committed, so a wire change can arrive
    // through a dependency.

    /// `RequestToMeltDesc { qid: Uuid::from_u128(42), amount: 2000 }`
    /// = String("00000000-0000-0000-0000-00000000002a") ++ u64le(2000)
    const REQUEST_TO_MELT_DESC_BORSH: &str = concat!(
        "24000000",
        "30303030303030302d303030302d303030302d303030302d303030303030303030303261",
        "d007000000000000",
    );

    /// `RequestToMintFromEBillDesc` = String(bill id) ++ i64le(deadline) ++ String(address)
    const REQUEST_TO_MINT_FROM_EBILL_DESC_BORSH: &str = concat!(
        "32000000",
        "626974637274424254356131654e5a387a45556b5532727070584244725a4a6a41526f",
        "78506b5a744267466f32524c7a3379",
        "00f1536500000000",
        "2c000000",
        "62637274317177353038643671656a7874646734793572337a6172766172793063357877",
        "376b796774303830",
    );

    const WIRE_CHANGED: &str = "the signed wire encoding changed: signatures made by other \
                                builds of this crate will no longer verify. If the change is \
                                deliberate, update the constant and treat it as a breaking \
                                wire change.";

    fn sample_request_to_melt() -> RequestToMeltDesc {
        RequestToMeltDesc {
            qid: uuid::Uuid::from_u128(42),
            amount: cashu::Amount::from(2000u64),
        }
    }

    fn sample_request_to_mint_from_ebill() -> RequestToMintFromEBillDesc {
        RequestToMintFromEBillDesc {
            ebill_id: BillId::from_str("bitcrtBBT5a1eNZ8zEUkU2rppXBDrZJjARoxPkZtBgFo2RLz3y")
                .expect("valid bill id"),
            deadline: crate::TStamp::from_unix_timestamp(1_700_000_000).expect("valid timestamp"),
            sweeping_address: String::from("bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080"),
        }
    }

    #[test]
    fn request_to_melt_desc_borsh_encoding_is_frozen() {
        let bytes = borsh::to_vec(&sample_request_to_melt()).expect("borsh serialize");
        assert_eq!(hex(&bytes), REQUEST_TO_MELT_DESC_BORSH, "{WIRE_CHANGED}");
    }

    #[test]
    fn request_to_mint_from_ebill_desc_borsh_encoding_is_frozen() {
        let bytes = borsh::to_vec(&sample_request_to_mint_from_ebill()).expect("borsh serialize");
        assert_eq!(
            hex(&bytes),
            REQUEST_TO_MINT_FROM_EBILL_DESC_BORSH,
            "{WIRE_CHANGED}"
        );
    }

    // A frozen encoding says the bytes did not move. It says nothing about the
    // deserializer, which is a separate `deserialize_with` that can drift out of step with
    // its partner. These two cover that half.

    #[test]
    fn request_to_melt_desc_borsh_roundtrip() {
        let desc = sample_request_to_melt();
        let bytes = borsh::to_vec(&desc).expect("borsh serialize");
        let back: RequestToMeltDesc = borsh::from_slice(&bytes).expect("borsh deserialize");
        assert_eq!(back.qid, desc.qid);
        assert_eq!(back.amount, desc.amount);
    }

    #[test]
    fn request_to_mint_from_ebill_desc_borsh_roundtrip() {
        let desc = sample_request_to_mint_from_ebill();
        let bytes = borsh::to_vec(&desc).expect("borsh serialize");
        let back: RequestToMintFromEBillDesc =
            borsh::from_slice(&bytes).expect("borsh deserialize");
        assert_eq!(back.ebill_id, desc.ebill_id);
        assert_eq!(back.deadline, desc.deadline);
        assert_eq!(back.sweeping_address, desc.sweeping_address);
    }
}
