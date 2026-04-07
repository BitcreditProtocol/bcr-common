// ----- standard library imports
// ----- extra library imports
use bitcoin::{bip32 as btc32, secp256k1 as secp};
use cashu::nut02 as cdk02;
use rand::RngCore;
// ----- local imports
use crate::core::{BillId, NodeId};

// ----- end imports

pub fn generate_random_keypair() -> secp::Keypair {
    let mut rng = rand::thread_rng();
    secp::Keypair::new(secp::global::SECP256K1, &mut rng)
}

pub fn random_bill_id() -> BillId {
    let keypair = generate_random_keypair();
    BillId::new(keypair.public_key(), bitcoin::Network::Testnet)
}

pub fn random_node_id() -> NodeId {
    let keypair = generate_random_keypair();
    NodeId::new(keypair.public_key(), bitcoin::Network::Testnet)
}

pub fn node_id_from_pub_key(pub_key: secp::PublicKey) -> NodeId {
    NodeId::new(pub_key, bitcoin::Network::Testnet)
}

pub fn generate_random_ecash_keyset() -> (cdk_common::mint::MintKeySetInfo, cashu::MintKeySet) {
    let path = btc32::DerivationPath::master();
    let mut random_seed = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut random_seed);
    let amounts = (0..10).map(|i| 2u64.pow(i)).collect::<Vec<u64>>();
    let set = cashu::MintKeySet::generate_from_seed(
        secp::global::SECP256K1,
        &random_seed,
        &amounts,
        cashu::CurrencyUnit::Sat,
        path.clone(),
        None,
        cdk02::KeySetVersion::Version00,
    );
    let info = cdk_common::mint::MintKeySetInfo {
        id: set.id,
        active: true,
        unit: cashu::CurrencyUnit::Sat,
        amounts,
        valid_from: 0,
        final_expiry: None,
        derivation_path_index: None,
        derivation_path: path,
        input_fee_ppk: 0,
        max_order: 10,
    };
    (info, set)
}

pub fn generate_random_ecash_proofs(
    keyset: &cashu::MintKeySet,
    amounts: &[cashu::Amount],
) -> Vec<cashu::Proof> {
    let mut proofs: Vec<cashu::Proof> = Vec::new();
    for amount in amounts {
        let keypair = keyset.keys.get(amount).expect("keys for amount");
        let secret = cashu::secret::Secret::new(rand::random::<u64>().to_string());
        let (b_, r) =
            cashu::dhke::blind_message(secret.as_bytes(), None).expect("cdk_dhke::blind_message");
        let c_ =
            cashu::dhke::sign_message(&keypair.secret_key, &b_).expect("cdk_dhke::sign_message");
        let c =
            cashu::dhke::unblind_message(&c_, &r, &keypair.public_key).expect("unblind_message");
        proofs.push(cashu::Proof::new(*amount, keyset.id, secret, c));
    }
    proofs
}

pub fn generate_random_ecash_blindedmessages(
    kid: cashu::Id,
    amounts: &[cashu::Amount],
) -> Vec<(
    cashu::BlindedMessage,
    cashu::secret::Secret,
    cashu::SecretKey,
)> {
    let mut blinds: Vec<(
        cashu::BlindedMessage,
        cashu::secret::Secret,
        cashu::SecretKey,
    )> = Vec::new();
    for amount in amounts {
        let secret = cashu::secret::Secret::new(rand::random::<u64>().to_string());
        let (b_, r) =
            cashu::dhke::blind_message(secret.as_bytes(), None).expect("cdk_dhke::blind_message");
        blinds.push((cashu::BlindedMessage::new(*amount, kid, b_), secret, r));
    }
    blinds
}

pub fn generate_ecash_signatures(
    keyset: &cashu::MintKeySet,
    amounts: &[cashu::Amount],
) -> Vec<cashu::BlindSignature> {
    let a_pk = cashu::PublicKey::from_hex(
        "0244e4420934530b2bdf5161f4c88b3c4f923db158741da51f3bb22b579495862e",
    )
    .unwrap();
    let mut signatures: Vec<cashu::BlindSignature> = Vec::new();
    for amount in amounts {
        signatures.push(cashu::BlindSignature {
            keyset_id: keyset.id,
            amount: *amount,
            c: a_pk,
            dleq: None,
        });
    }
    signatures
}
