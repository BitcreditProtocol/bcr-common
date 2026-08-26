// ----- standard library imports
use std::time::Duration;
// ----- extra library imports
use async_nats::ConnectOptions;
// ----- local modules
mod error;
pub mod model;
mod nats_client;
mod sign_client;
// ----- end imports

pub use error::*;
pub use nats_client::ClowderNatsClient;
pub use reqwest::Url;
pub use sign_client::SignatoryNatsClient;

pub fn nats_options(timeout: Duration, nkey_seed: Option<&str>) -> ConnectOptions {
    let options = ConnectOptions::new().request_timeout(Some(timeout));
    match nkey_seed {
        Some(seed) => options.nkey(seed.to_owned()),
        None => options,
    }
}
