// ----- standard library imports
// ----- extra library imports
use bitcoin::secp256k1 as secp;
// ----- local modules
use crate::{
    core::test_utils::{generate_random_keypair, node_id_from_pub_key},
    wire::{bill::BillIdentParticipant, contact::ContactType, identity::PostalAddress},
};

// ----- end imports

pub fn random_identity_public_data() -> (secp::Keypair, BillIdentParticipant) {
    let keypair = generate_random_keypair();
    let sample = [
        BillIdentParticipant {
            t: ContactType::Person,
            email: Some(String::from("Carissa@kemp.com")),
            name: String::from("Carissa Kemp"),
            node_id: node_id_from_pub_key(keypair.public_key()),
            postal_address: PostalAddress {
                country: String::from("Austria"),
                city: String::from("Vorarlberg"),
                zip: Some(String::from("5196")),
                address: String::from("Auf der Stift 17c"),
            },
            nostr_relays: vec![],
        },
        BillIdentParticipant {
            t: ContactType::Person,
            email: Some(String::from("alana@carrillo.com")),
            name: String::from("Alana Carrillo"),
            node_id: node_id_from_pub_key(keypair.public_key()),
            postal_address: PostalAddress {
                country: String::from("Spain"),
                city: String::from("Madrid"),
                zip: Some(String::from("81015")),
                address: String::from("Paseo José Emilio Ruíz 69"),
            },
            nostr_relays: vec![],
        },
        BillIdentParticipant {
            t: ContactType::Person,
            email: Some(String::from("geremia@pisani.com")),
            name: String::from("Geremia Pisani"),
            node_id: node_id_from_pub_key(keypair.public_key()),
            postal_address: PostalAddress {
                country: String::from("Italy"),
                city: String::from("Firenze"),
                zip: Some(String::from("50141")),
                address: String::from("Piazza Principale Umberto 148"),
            },
            nostr_relays: vec![],
        },
        BillIdentParticipant {
            t: ContactType::Person,
            email: Some(String::from("andreas@koenig.com")),
            name: String::from("Andreas Koenig"),
            node_id: node_id_from_pub_key(keypair.public_key()),
            postal_address: PostalAddress {
                country: String::from("Austria"),
                city: String::from("Lorberhof"),
                zip: Some(String::from("9556")),
                address: String::from("Haiden 96"),
            },
            nostr_relays: vec![],
        },
        BillIdentParticipant {
            t: ContactType::Company,
            email: Some(String::from("logistilla@fournier.com")),
            name: String::from("Logistilla Fournier"),
            node_id: node_id_from_pub_key(keypair.public_key()),
            postal_address: PostalAddress {
                country: String::from("France"),
                city: String::from("Toulouse"),
                zip: Some(String::from("31000")),
                address: String::from("25, rue Pierre de Coubertin"),
            },
            nostr_relays: vec![],
        },
        BillIdentParticipant {
            t: ContactType::Company,
            email: Some(String::from("moonlimited@ltd.com")),
            name: String::from("Moon Limited"),
            node_id: node_id_from_pub_key(keypair.public_key()),
            postal_address: PostalAddress {
                country: String::from("USA"),
                city: String::from("New York"),
                zip: Some(String::from("86659-2593")),
                address: String::from("3443 Joanny Bypass"),
            },
            nostr_relays: vec![],
        },
        BillIdentParticipant {
            t: ContactType::Company,
            email: Some(String::from("blanco@spa.com")),
            name: String::from("Blanco y Asoc."),
            node_id: node_id_from_pub_key(keypair.public_key()),
            postal_address: PostalAddress {
                country: String::from("Argentina"),
                city: String::from("Puerto Clara"),
                zip: Some(String::from("38074")),
                address: String::from("Isidora 96 0 7"),
            },
            nostr_relays: vec![],
        },
        BillIdentParticipant {
            t: ContactType::Company,
            email: Some(String::from("alexanderurner@grimm.com")),
            name: String::from("Grimm GmbH"),
            node_id: node_id_from_pub_key(keypair.public_key()),
            postal_address: PostalAddress {
                country: String::from("Austria"),
                city: String::from("Perg"),
                zip: Some(String::from("3512")),
                address: String::from("Barthring 342"),
            },
            nostr_relays: vec![],
        },
        BillIdentParticipant {
            t: ContactType::Company,
            email: Some(String::from("antoniosegovia@santiago.com")),
            name: String::from("Empresa Santiago"),
            node_id: node_id_from_pub_key(keypair.public_key()),
            postal_address: PostalAddress {
                country: String::from("Spain"),
                city: String::from("Vall Juarez"),
                zip: Some(String::from("88191")),
                address: String::from("Avinguida José Antonio, 20"),
            },
            nostr_relays: vec![],
        },
        BillIdentParticipant {
            t: ContactType::Company,
            email: Some(String::from("santoro_group@spa.com")),
            name: String::from("Santoro Group"),
            node_id: node_id_from_pub_key(keypair.public_key()),
            postal_address: PostalAddress {
                country: String::from("Italy"),
                city: String::from("Prunetta"),
                zip: Some(String::from("51020")),
                address: String::from("Corso Vittorio Emanuele, 90"),
            },
            nostr_relays: vec![],
        },
    ];
    let mut rng = rand::thread_rng();
    let random_index = rand::Rng::gen_range(&mut rng, 0..sample.len());
    let random_identity = sample[random_index].clone();
    (keypair, random_identity)
}
