// ----- standard library imports
// ----- extra library imports
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::wire::clowder::ClowderNodeInfo;
// ----- local imports

// ----- end imports

/// Version information for the mint
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VersionInfo {
    /// Wildcat version (from PKG_VERSION)
    pub wildcat: String,
    /// bcr-ebill-core version
    pub bcr_ebill_core: Option<String>,
    /// cdk-mintd version (from upstream mint info)
    pub cdk_mintd: Option<String>,
}

/// Mint information including network, build time, versions, and uptime
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WildcatInfo {
    /// Bitcoin network (mainnet, testnet, signet, regtest)
    #[schema(value_type = String)]
    pub network: bitcoin::Network,
    /// Build timestamp
    pub build_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Service uptime, last started
    pub uptime_timestamp: u64,
    /// Version information
    pub versions: VersionInfo,
    /// Clowder
    pub clowder: ClowderNodeInfo,
}
