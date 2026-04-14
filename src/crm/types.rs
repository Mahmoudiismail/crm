use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportType {
    All,
    Tickets,
    Calls,
    Leads,
    None,
}

impl Default for ReportType {
    fn default() -> Self {
        Self::All
    }
}
