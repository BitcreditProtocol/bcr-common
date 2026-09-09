// ----- standard library imports
// ----- extra library imports
// ----- local modules
pub mod admin;
#[cfg(feature = "clwdr-client")]
pub mod clowder;
pub mod ebill;
pub mod mint;

// ----- end imports

pub use reqwest::Url;

// Re-export admin clients at the old paths for backward compatibility.
pub use admin::core;
pub use admin::quote;
pub use admin::treasury;

const CURRENCY_UNIT: cashu::CurrencyUnit = cashu::CurrencyUnit::Sat;

// This is a workaround for Android targets, since reqwest 0.13 added complexity and issues by using
// rust-platform-verifier (https://github.com/rustls/rustls-platform-verifier#android)
#[cfg(all(feature = "webpki-roots", not(target_arch = "wasm32")))]
pub fn reqwest_client_builder() -> reqwest::ClientBuilder {
    let certificates = webpki_root_certs::TLS_SERVER_ROOT_CERTS.iter().map(|cert| {
        reqwest::Certificate::from_der(cert.as_ref())
            .expect("webpki root certificate must be valid DER")
    });

    reqwest::Client::builder().tls_certs_only(certificates)
}

#[cfg(not(all(feature = "webpki-roots", not(target_arch = "wasm32"))))]
pub fn reqwest_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
}

/// Centralized client-builder
pub fn reqwest_client() -> reqwest::Client {
    reqwest_client_builder()
        .build()
        .expect("failed to build reqwest client")
}
