// ----- standard library imports
use std::collections::HashMap;
// ----- extra library imports
use serde::{Deserialize, Serialize};
// ----- local imports

// ----- end imports

/// --------------------------- admin notification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NotificationRequest {
    NewEbillQuote {
        #[serde(default)]
        fields: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationResponse {}
