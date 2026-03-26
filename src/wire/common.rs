// ----- standard library imports
// ----- extra library imports
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
// ----- local imports

// ----- end imports

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T: ToSchema> {
    pub data: Vec<T>,
    pub total: u64,
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
pub struct Pagination {
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum ProtestStatus {
    Resolved,
    Rabid,
}
