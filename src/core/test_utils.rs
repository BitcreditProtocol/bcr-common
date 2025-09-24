// ----- standard library imports
// ----- extra library imports
use bitcoin::secp256k1;
// ----- local imports
use crate::core::{BillId, NodeId};

// ----- end imports

pub fn generate_random_keypair() -> secp256k1::Keypair {
    let mut rng = rand::thread_rng();
    secp256k1::Keypair::new(secp256k1::global::SECP256K1, &mut rng)
}

pub fn random_bill_id() -> BillId {
    let keypair = generate_random_keypair();
    BillId::new(keypair.public_key().into(), bitcoin::Network::Testnet)
}

pub fn random_node_id() -> NodeId {
    let keypair = generate_random_keypair();
    NodeId::new(keypair.public_key(), bitcoin::Network::Testnet)
}

pub fn node_id_from_pub_key(pub_key: secp256k1::PublicKey) -> NodeId {
    NodeId::new(pub_key, bitcoin::Network::Testnet)
}
