// ----- standard library imports
// ----- extra library imports
// ----- local modules
pub mod admin;
pub mod ebill;
pub mod mint;

// ----- end imports

pub use reqwest::Url;

// Re-export admin clients at the old paths for backward compatibility.
pub use admin::core;
pub use admin::quote;
pub use admin::treasury;

const CURRENCY_UNIT: cashu::CurrencyUnit = cashu::CurrencyUnit::Sat;
