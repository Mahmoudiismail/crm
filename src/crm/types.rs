use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
#[derive(clap::ValueEnum)]
pub enum ReportType {
    #[default]
    All,
    Tickets,
    Calls,
    Leads,
    None,
}
