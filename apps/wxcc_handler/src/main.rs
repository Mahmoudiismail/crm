use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use crm_tool::manifest::{AppArg, AppManifest, ArgType};
use crm_tool::utils::{executable_dir, intercept_manifest, parse_log_level, setup_logging_with_levels, InterceptResult};
use std::path::{Path, PathBuf};
use tracing::{error, info};
use wxcc_handler::wxcc::{auth, config::WxccConfig};

#[derive(Debug, Parser)]
#[command(name = "wxcc_handler", version, about = "Webex Contact Center process handler")]
struct Cli {
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}
#[derive(Debug, Subcommand)]
enum Command {
    Config { #[command(subcommand)] command: ConfigCommand },
    Auth { #[command(subcommand)] command: AuthCommand },
}
#[derive(Debug, Subcommand)]
enum ConfigCommand { Init, Check }
#[derive(Debug, Subcommand)]
enum AuthCommand { AuthorizeUrl, ExchangeCode(ExchangeCodeArgs), Refresh, Status }
#[derive(Debug, Args)]
struct ExchangeCodeArgs { #[arg(long)] code: String, #[arg(long)] state: String }

fn resolve_config_path(config: Option<&Path>, exe_dir: &Path) -> PathBuf {
    match config {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => exe_dir.join(path),
        None => exe_dir.join("wxcc_config.json"),
    }
}
fn get_manifest() -> AppManifest {
    AppManifest {
        plugin_api_version: Some(crm_tool::manifest::PROCESS_PLUGIN_API_VERSION),
        name: "Webex Contact Center Handler".into(),
        description: "Official-API process handler for Webex Contact Center.".into(),
        arguments: vec![AppArg::new("--config", ArgType::String)],
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    if let InterceptResult::ExitSuccessfully = intercept_manifest(get_manifest()) { return Ok(()); }
    let cli = Cli::parse();
    let exe_dir = executable_dir()?;
    let config_path = resolve_config_path(cli.config.as_deref(), &exe_dir);
    let config = WxccConfig::load(&config_path.to_string_lossy()).context("Failed to load WXCC Handler configuration")?;
    let _guard = setup_logging_with_levels("wxcc_handler", parse_log_level(&config.log_stdout_level)?, parse_log_level(&config.log_file_level)?)
        .context("Failed to initialize WXCC Handler logging")?;
    info!("WXCC Handler command started");

    let result = match cli.command {
        Command::Config { command: ConfigCommand::Init } => {
            println!("Configuration is ready: {}", config_path.display()); Ok(())
        }
        Command::Config { command: ConfigCommand::Check } => {
            config.validate(false)?; println!("Configuration is valid."); Ok(())
        }
        Command::Auth { command: AuthCommand::AuthorizeUrl } => {
            let (url, state) = auth::authorization_url(&config_path, config.clone())?;
            println!("Open this URL in your browser:\n{url}\n");
            println!("OAuth state (compare with the state query parameter after redirect):\n{state}");
            Ok(())
        }
        Command::Auth { command: AuthCommand::ExchangeCode(args) } => {
            auth::exchange_code(&config_path, config, &args.code, &args.state).await?;
            println!("Webex authorization completed. Tokens were saved locally."); Ok(())
        }
        Command::Auth { command: AuthCommand::Refresh } => {
            auth::refresh_token(&config_path, config).await?;
            println!("Webex access token refreshed and saved."); Ok(())
        }
        Command::Auth { command: AuthCommand::Status } => {
            if config.has_access_token() {
                println!("Authorized; access token expiry (Unix UTC): {}.", config.access_token_expires_at.unwrap_or_default());
            } else if config.refresh_token.is_some() {
                println!("Refresh token is present; an access token refresh is required.");
            } else { println!("Not authorized. Run wxcc_handler auth authorize-url."); }
            Ok(())
        }
    };
    if let Err(err) = result { error!("Command failed: {err:#}"); return Err(err); }
    info!("WXCC Handler command completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relative_and_default_config_paths_use_executable_directory() {
        let base = Path::new("C:/apps/wxcc");
        assert_eq!(resolve_config_path(Some(Path::new("settings.json")), base), base.join("settings.json"));
        assert_eq!(resolve_config_path(None, base), base.join("wxcc_config.json"));
    }
    #[test]
    fn absolute_config_path_is_preserved() {
        let path = if cfg!(windows) { PathBuf::from("C:/data/wxcc.json") } else { PathBuf::from("/data/wxcc.json") };
        assert_eq!(resolve_config_path(Some(&path), Path::new(".")), path);
    }
}
