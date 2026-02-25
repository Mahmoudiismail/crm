use clap::{Parser, ValueEnum};

/// CRM Tool — CLI for fetching CRM reports via AWS Cognito authentication
#[derive(Parser, Debug)]
#[command(name = "crm_tool", version, about, long_about = None)]
pub struct CliArgs {
    /// Path to JSON config file
    #[arg(long, default_value = "config.json")]
    pub config: String,

    /// AWS region
    #[arg(long)]
    pub region: Option<String>,

    /// Cognito User Pool ID
    #[arg(long)]
    pub user_pool_id: Option<String>,

    /// Cognito App Client ID
    #[arg(long)]
    pub client_id: Option<String>,

    /// Cognito username / phone
    #[arg(long)]
    pub username: Option<String>,

    /// Cognito password
    #[arg(long)]
    pub password: Option<String>,

    /// Email for CRM report requests
    #[arg(long)]
    pub email: Option<String>,

    /// Start date for tickets/leads (YYYY-MM-DD)
    #[arg(long)]
    pub from_date: Option<String>,

    /// Start date for call logs (YYYY-MM-DD)
    #[arg(long)]
    pub calls_from_date: Option<String>,

    /// End date (YYYY-MM-DD), defaults to today
    #[arg(long)]
    pub to_date: Option<String>,

    /// Report type to fetch
    #[arg(long, value_enum, default_value = "all")]
    pub report: ReportType,

    /// Save JSON output to file
    #[arg(long)]
    pub output: Option<String>,

    /// Disable TLS certificate verification
    #[arg(long, default_value_t = false)]
    pub no_verify_ssl: bool,

    /// Use cached token, skip Cognito login
    #[arg(long, default_value_t = false)]
    pub skip_login: bool,

    /// Persist password/tokens to config
    #[arg(long)]
    pub remember_secrets: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ReportType {
    All,
    Tickets,
    Calls,
    Leads,
    None,
}
