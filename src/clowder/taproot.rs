// ----- standard library imports
// ----- extra library imports
use bitcoin::XOnlyPublicKey;
use bitcoin::hashes::{Hash, HashEngine, sha256};
use bitcoin::key::TapTweak;
use bitcoin::opcodes::all::{OP_CHECKSIG, OP_CHECKSIGVERIFY, OP_CSV};
use bitcoin::script::Builder as ScriptBuilder;
use bitcoin::secp256k1::{Parity, PublicKey};
use bitcoin::taproot::{ControlBlock, LeafVersion, TapLeafHash, TaprootBuilder, TaprootSpendInfo};
use bitcoin::{Address, Network, ScriptBuf};
use uuid::Uuid;
// ----- local imports
use super::{Error, Result};
// ----- end imports

/// BIP 341 taproot tweak
fn taproot_tweak_pubkey(pubkey: [u8; 32], merkle_root: &[u8]) -> Result<(bool, [u8; 32])> {
    // Used in frost_secp256k1_tr, and as a result when aggregating and signing we use the same prefix
    let prefix = sha256::Hash::hash(b"TapTweak");

    let mut engine = sha256::Hash::engine();
    engine.input(prefix.as_ref());
    engine.input(prefix.as_ref());
    engine.input(&pubkey);
    engine.input(merkle_root);
    let tweak_hash = sha256::Hash::from_engine(engine);

    let mut tweak_bytes = [0u8; 32];
    tweak_bytes.copy_from_slice(tweak_hash.as_ref());
    let tweak = bitcoin::secp256k1::Scalar::from_be_bytes(tweak_bytes)?;

    let mut pubkey_bytes = [0u8; 33];
    // SEC1 compressed-point prefix for even y-coordinate
    pubkey_bytes[0] = 0x02;
    pubkey_bytes[1..].copy_from_slice(&pubkey);
    let pubkey_point = PublicKey::from_slice(&pubkey_bytes)?;

    let tweaked = pubkey_point.add_exp_tweak(bitcoin::secp256k1::SECP256K1, &tweak)?;
    let (xonly, parity) = tweaked.x_only_public_key();

    Ok((parity == Parity::Odd, xonly.serialize()))
}

pub fn taproot_tweak(
    pubkey: bitcoin::secp256k1::PublicKey,
    merkle_root: &[u8],
) -> Result<(Parity, XOnlyPublicKey)> {
    let pubkey_bytes = pubkey.serialize()[1..]
        .try_into()
        .map_err(|_| Error::InvalidPubkey)?;
    let (y_odd, tweaked_x) = taproot_tweak_pubkey(pubkey_bytes, merkle_root)?;

    let xonly = XOnlyPublicKey::from_slice(&tweaked_x)?;
    let parity = if y_odd { Parity::Odd } else { Parity::Even };

    Ok((parity, xonly))
}

const NUMS_TAG: &[u8] = b"Clowder/unspendable";

pub fn nums_point() -> XOnlyPublicKey {
    use std::sync::LazyLock;
    static NUMS_BASE: LazyLock<XOnlyPublicKey> = LazyLock::new(|| {
        let hash = sha256::Hash::hash(NUMS_TAG);
        let mut bytes = [0u8; 33];
        // SEC1 compressed-point prefix for even y-coordinate
        bytes[0] = 0x02;
        bytes[1..].copy_from_slice(hash.as_ref());
        loop {
            match PublicKey::from_slice(&bytes) {
                Ok(pk) => return pk.x_only_public_key().0,
                Err(_) => {
                    let next = sha256::Hash::hash(&bytes[1..]);
                    bytes[1..].copy_from_slice(next.as_ref());
                }
            }
        }
    });
    *NUMS_BASE
}

pub fn derive_nums(tweak: &[u8; 32]) -> Result<XOnlyPublicKey> {
    let base = nums_point();
    let base_key = base.public_key(bitcoin::secp256k1::Parity::Even);
    let (_parity, tweaked) = taproot_tweak(base_key, tweak)?;
    Ok(tweaked)
}

const CLOWDER_TWEAK_TAG: &[u8] = b"Clowder/tweak";

fn clowder_tagged_hash(purpose: &[u8], key: &XOnlyPublicKey, payload: &[u8]) -> [u8; 32] {
    let tag = sha256::Hash::hash(CLOWDER_TWEAK_TAG);
    let mut engine = sha256::Hash::engine();
    engine.input(tag.as_ref());
    engine.input(tag.as_ref());
    engine.input(&key.serialize());
    engine.input(purpose);
    engine.input(payload);
    sha256::Hash::from_engine(engine).to_byte_array()
}

pub fn derive_receive_tweak(aggregated_key: &XOnlyPublicKey, uuid: &Uuid) -> [u8; 32] {
    clowder_tagged_hash(b"receive", aggregated_key, uuid.as_bytes())
}

pub fn derive_change_tweak(aggregated_key: &XOnlyPublicKey) -> [u8; 32] {
    clowder_tagged_hash(b"change", aggregated_key, &[])
}

pub fn build_beta_script(frost_agg_key: &XOnlyPublicKey) -> ScriptBuf {
    ScriptBuilder::new()
        .push_x_only_key(frost_agg_key)
        .push_opcode(OP_CHECKSIG)
        .into_script()
}

pub fn build_alpha_script(alpha_key: &XOnlyPublicKey, timelock_blocks: u32) -> ScriptBuf {
    ScriptBuilder::new()
        .push_x_only_key(alpha_key)
        .push_opcode(OP_CHECKSIGVERIFY)
        .push_int(timelock_blocks as i64)
        .push_opcode(OP_CSV)
        .into_script()
}

pub struct TapTreeInfo {
    pub tap_info: TaprootSpendInfo,
    pub beta_script: ScriptBuf,
    pub alpha_script: ScriptBuf,
    pub beta_leaf_hash: TapLeafHash,
    pub alpha_leaf_hash: TapLeafHash,
}

impl TapTreeInfo {
    pub fn output_key(&self) -> XOnlyPublicKey {
        self.tap_info.output_key().to_x_only_public_key()
    }

    pub fn beta_control_block(&self) -> Option<ControlBlock> {
        self.tap_info
            .control_block(&(self.beta_script.clone(), LeafVersion::TapScript))
    }

    pub fn alpha_control_block(&self) -> Option<ControlBlock> {
        self.tap_info
            .control_block(&(self.alpha_script.clone(), LeafVersion::TapScript))
    }

    pub fn address(&self, network: Network) -> Address {
        let tweaked =
            bitcoin::key::UntweakedPublicKey::from(self.output_key()).dangerous_assume_tweaked();
        Address::p2tr_tweaked(tweaked, network)
    }
}

pub fn build_tap_tree(
    frost_agg_key: &XOnlyPublicKey,
    alpha_key: &XOnlyPublicKey,
    timelock_blocks: u32,
    internal_key: &XOnlyPublicKey,
) -> Result<TapTreeInfo> {
    let beta_script = build_beta_script(frost_agg_key);
    let alpha_script = build_alpha_script(alpha_key, timelock_blocks);

    let beta_leaf_hash = TapLeafHash::from_script(&beta_script, LeafVersion::TapScript);
    let alpha_leaf_hash = TapLeafHash::from_script(&alpha_script, LeafVersion::TapScript);

    let tap_info = TaprootBuilder::new()
        .add_leaf(1, beta_script.clone())?
        .add_leaf(1, alpha_script.clone())?
        .finalize(bitcoin::secp256k1::SECP256K1, *internal_key)
        .map_err(|_| Error::IncompleteTaprootTree)?;

    Ok(TapTreeInfo {
        tap_info,
        beta_script,
        alpha_script,
        beta_leaf_hash,
        alpha_leaf_hash,
    })
}

pub fn build_base_tap_tree(
    frost_agg_key: &XOnlyPublicKey,
    alpha_key: &XOnlyPublicKey,
    timelock_blocks: u32,
) -> Result<TapTreeInfo> {
    build_tap_tree(frost_agg_key, alpha_key, timelock_blocks, &nums_point())
}

pub fn build_tap_tree_for_tweak(
    frost_agg_key: &XOnlyPublicKey,
    alpha_key: &XOnlyPublicKey,
    timelock_blocks: u32,
    tweak: &[u8; 32],
) -> Result<TapTreeInfo> {
    let nums = derive_nums(tweak)?;
    build_tap_tree(frost_agg_key, alpha_key, timelock_blocks, &nums)
}

pub fn derive_receive_address(
    frost_agg_key: &XOnlyPublicKey,
    alpha_key: &XOnlyPublicKey,
    timelock_blocks: u32,
    uuid: &Uuid,
    network: Network,
) -> Result<Address> {
    let tweak = derive_receive_tweak(frost_agg_key, uuid);
    Ok(
        build_tap_tree_for_tweak(frost_agg_key, alpha_key, timelock_blocks, &tweak)?
            .address(network),
    )
}

pub fn derive_change_address(
    frost_agg_key: &XOnlyPublicKey,
    alpha_key: &XOnlyPublicKey,
    timelock_blocks: u32,
    network: Network,
) -> Result<Address> {
    let tweak = derive_change_tweak(frost_agg_key);
    Ok(
        build_tap_tree_for_tweak(frost_agg_key, alpha_key, timelock_blocks, &tweak)?
            .address(network),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::SECP256K1;

    #[test]
    fn test_nums_point_is_deterministic() {
        let p1 = nums_point();
        let p2 = nums_point();
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_derive_nums_different_tweaks() {
        let t1 = [1u8; 32];
        let t2 = [2u8; 32];
        let p1 = derive_nums(&t1).unwrap();
        let p2 = derive_nums(&t2).unwrap();
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_build_tap_tree() {
        let (_, frost_key) =
            SECP256K1.generate_keypair(&mut bitcoin::secp256k1::rand::thread_rng());
        let (_, alpha_key) =
            SECP256K1.generate_keypair(&mut bitcoin::secp256k1::rand::thread_rng());
        let frost_xonly = frost_key.x_only_public_key().0;
        let alpha_xonly = alpha_key.x_only_public_key().0;
        let internal = nums_point();

        let info = build_tap_tree(&frost_xonly, &alpha_xonly, 144, &internal).unwrap();

        assert!(info.beta_control_block().is_some());
        assert!(info.alpha_control_block().is_some());
        assert_ne!(info.beta_leaf_hash, info.alpha_leaf_hash);
    }

    #[test]
    fn test_per_address_nums_derivation() {
        let tweak_a = sha256::Hash::hash(b"address_a").to_byte_array();
        let tweak_b = sha256::Hash::hash(b"address_b").to_byte_array();

        let (_, frost_key) =
            SECP256K1.generate_keypair(&mut bitcoin::secp256k1::rand::thread_rng());
        let (_, alpha_key) =
            SECP256K1.generate_keypair(&mut bitcoin::secp256k1::rand::thread_rng());
        let frost_xonly = frost_key.x_only_public_key().0;
        let alpha_xonly = alpha_key.x_only_public_key().0;

        let nums_a = derive_nums(&tweak_a).unwrap();
        let nums_b = derive_nums(&tweak_b).unwrap();

        let tree_a = build_tap_tree(&frost_xonly, &alpha_xonly, 144, &nums_a).unwrap();
        let tree_b = build_tap_tree(&frost_xonly, &alpha_xonly, 144, &nums_b).unwrap();

        assert_ne!(tree_a.output_key(), tree_b.output_key());
    }

    #[test]
    fn test_address_derivation() {
        let mut rng = bitcoin::secp256k1::rand::thread_rng();
        let frost = SECP256K1.generate_keypair(&mut rng).1.x_only_public_key().0;
        let alpha = SECP256K1.generate_keypair(&mut rng).1.x_only_public_key().0;
        let net = Network::Regtest;

        let r1 = derive_receive_address(&frost, &alpha, 144, &Uuid::from_u128(1), net).unwrap();
        let r2 = derive_receive_address(&frost, &alpha, 144, &Uuid::from_u128(2), net).unwrap();
        let c1 = derive_change_address(&frost, &alpha, 144, net).unwrap();
        let c2 = derive_change_address(&frost, &alpha, 144, net).unwrap();

        let frost2 = SECP256K1.generate_keypair(&mut rng).1.x_only_public_key().0;
        let r3 = derive_receive_address(&frost2, &alpha, 144, &Uuid::from_u128(1), net).unwrap();

        assert_ne!(r1, r2);
        assert_ne!(r1, c1);
        assert_eq!(c1, c2);
        assert_ne!(r1, r3);
        assert!(r1.to_string().starts_with("bcrt1p"));
    }
}
