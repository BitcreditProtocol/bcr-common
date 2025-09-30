// ----- standard library imports
// ----- extra library imports
// ----- local modules
mod client;
pub mod core;
pub mod wire;

// ----- end imports

pub use client::keys::Client as KeysClient;
pub use client::quote::Client as QuoteClient;
pub use client::swap::Client as SwapClient;

#[cfg(feature = "test-utils")]
pub use core::test_utils as core_tests;
#[cfg(feature = "test-utils")]
pub use wire::test_utils as wire_tests;
