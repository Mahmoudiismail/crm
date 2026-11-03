use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TeamMapping {
    #[serde(alias = "Team Name")]
    pub team_name: String,
    #[serde(alias = "Receiver Name")]
    pub receiver_name: Option<String>,
    #[serde(alias = "To Emails")]
    pub to_emails: Option<String>,
    #[serde(alias = "CC")]
    pub cc: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TicketRow {
    pub ticket_id: String,
    pub assignee: String,
    pub ticket_type: String,
    pub ticket_subtype: String,
    pub ticket_category: String,
    pub status: String,
    pub branch: String,
    pub team: String,
    pub created_at_dt: Option<NaiveDate>,
    pub original_row: csv::StringRecord,
}
