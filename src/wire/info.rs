// ----- standard library imports
// ----- extra library imports
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// ----- local imports

// ----- end imports

/// Version information for the mint
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VersionInfo {
    /// Wildcat version
    pub wildcat: String,
    /// bcr-ebill-core version
    pub bcr_ebill_core: String,
    /// cdk-mintd version
    pub cdk_mintd: String,
    /// Clowder version
    pub clowder: String,
}

/// Mint information including network, build time, versions, and uptime
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WildcatInfo {
    /// Bitcoin network (mainnet, testnet, signet, regtest)
    #[schema(value_type = String)]
    pub network: bitcoin::Network,
    /// Build timestamp
    pub build_time: chrono::DateTime<chrono::Utc>,
    /// Service uptime, last started
    pub uptime_timestamp: chrono::DateTime<chrono::Utc>,
    /// Version information
    pub versions: VersionInfo,
    /// Clowder node id
    #[schema(value_type = String)]
    pub clowder_node_id: bitcoin::secp256k1::PublicKey,
    /// Clowder change address
    #[schema(value_type = String)]
    pub clowder_change_address: bitcoin::address::Address<bitcoin::address::NetworkUnchecked>,
}
