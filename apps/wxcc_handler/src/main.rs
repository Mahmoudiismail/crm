use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use crm_tool::manifest::{AppArg, AppManifest, ArgType};
use crm_tool::utils::{
    executable_dir, intercept_manifest, parse_log_level, setup_logging_with_levels, InterceptResult,
};
use std::{collections::HashMap, path::{Path, PathBuf}};
use tracing::{error, info};
use wxcc_handler::wxcc::{auth, config::WxccConfig, data};

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum RunnerAction {
    ConfigInit,
    ConfigCheck,
    AuthAuthorizeUrl,
    AuthExchangeCode,
    AuthRefresh,
    AuthStatus,
    DataCollect,
}

#[derive(Debug, Parser)]
#[command(
    name = "wxcc_handler",
    version,
    about = "Webex Contact Center process handler"
)]
struct Cli {
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    /// Select an operation when launched by Runner.
    #[arg(long, value_enum, conflicts_with = "command")]
    action: Option<RunnerAction>,
    /// OAuth authorization code (Runner action: auth-exchange-code).
    #[arg(long, requires = "action")]
    code: Option<String>,
    /// OAuth state returned with the authorization code.
    #[arg(long, requires = "action")]
    state: Option<String>,
    /// CSV destination (Runner action: data-collect).
    #[arg(long, global = true)]
    output: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    Data {
        #[command(subcommand)]
        command: DataCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    Init,
    Check,
}

#[derive(Debug, Subcommand)]
enum AuthCommand {
    AuthorizeUrl,
    ExchangeCode(ExchangeCodeArgs),
    Refresh,
    Status,
}

#[derive(Debug, Subcommand)]
enum DataCommand {
    Collect,
}

#[derive(Debug, Args)]
struct ExchangeCodeArgs {
    #[arg(long)]
    code: String,
    #[arg(long)]
    state: String,
}

fn resolve_config_path(config: Option<&Path>, exe_dir: &Path) -> PathBuf {
    match config {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => exe_dir.join(path),
        None => exe_dir.join("wxcc_config.json"),
    }
}

fn resolve_output_path(output: Option<&Path>, exe_dir: &Path) -> PathBuf {
    match output {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => exe_dir.join(path),
        None => exe_dir.join("wxcc_data.csv"),
    }
}

fn get_manifest() -> AppManifest {
    let action_values = [
        ("config-init", RunnerAction::ConfigInit),
        ("config-check", RunnerAction::ConfigCheck),
        ("auth-authorize-url", RunnerAction::AuthAuthorizeUrl),
        ("auth-exchange-code", RunnerAction::AuthExchangeCode),
        ("auth-refresh", RunnerAction::AuthRefresh),
        ("auth-status", RunnerAction::AuthStatus),
        ("data-collect", RunnerAction::DataCollect),
    ];
    let action_options = action_values.iter().map(|(name, _)| *name);
    let depends_on = |values: &[&str]| {
        HashMap::from([("--action".to_string(), values.iter().map(|v| (*v).to_string()).collect())])
    };
    AppManifest {
        plugin_api_version: Some(crm_tool::manifest::PROCESS_PLUGIN_API_VERSION),
        name: "Webex Contact Center Handler".into(),
        description: "Official-API process handler for Webex Contact Center.".into(),
        arguments: vec![
            AppArg::new("--config", ArgType::String),
            AppArg::new("--action", ArgType::List).required(true).options(action_options),
            AppArg::new("--code", ArgType::String).required(true).depends_on(depends_on(&["auth-exchange-code"])),
            AppArg::new("--state", ArgType::String).required(true).depends_on(depends_on(&["auth-exchange-code"])),
            AppArg::new("--output", ArgType::String).depends_on(depends_on(&["data-collect"])),
        ],
    }
}

fn command_from_runner_action(
    action: RunnerAction,
    code: Option<&str>,
    state: Option<&str>,
) -> Result<Command> {
    Ok(match action {
        RunnerAction::ConfigInit => Command::Config { command: ConfigCommand::Init },
        RunnerAction::ConfigCheck => Command::Config { command: ConfigCommand::Check },
        RunnerAction::AuthAuthorizeUrl => Command::Auth { command: AuthCommand::AuthorizeUrl },
        RunnerAction::AuthExchangeCode => Command::Auth {
            command: AuthCommand::ExchangeCode(ExchangeCodeArgs {
                code: code.context("--code is required for --action auth-exchange-code")?.to_string(),
                state: state.context("--state is required for --action auth-exchange-code")?.to_string(),
            }),
        },
        RunnerAction::AuthRefresh => Command::Auth { command: AuthCommand::Refresh },
        RunnerAction::AuthStatus => Command::Auth { command: AuthCommand::Status },
        RunnerAction::DataCollect => Command::Data { command: DataCommand::Collect },
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    if let InterceptResult::ExitSuccessfully = intercept_manifest(get_manifest()) {
        return Ok(());
    }
    let cli = Cli::parse();
    let command = match (cli.action, cli.command.as_ref()) {
        (Some(_), Some(_)) => anyhow::bail!("Use either --action or a CLI subcommand, not both"),
        (Some(action), None) => command_from_runner_action(action, cli.code.as_deref(), cli.state.as_deref())?,
        (None, Some(command)) => command.clone(),
        (None, None) => anyhow::bail!("An operation is required. Use --help to see available commands."),
    };
    let exe_dir = executable_dir()?;
    let config_path = resolve_config_path(cli.config.as_deref(), &exe_dir);
    let config = WxccConfig::load(&config_path.to_string_lossy())
        .context("Failed to load WXCC Handler configuration")?;
    let _guard = setup_logging_with_levels(
        "wxcc_handler",
        parse_log_level(&config.log_stdout_level)?,
        parse_log_level(&config.log_file_level)?,
    )
    .context("Failed to initialize WXCC Handler logging")?;
    info!("WXCC Handler command started");

    let result = match command {
        Command::Config { command: ConfigCommand::Init } => {
            println!("Configuration is ready: {}", config_path.display());
            Ok(())
        }
        Command::Config { command: ConfigCommand::Check } => {
            config.validate(false)?;
            println!("Configuration is valid.");
            Ok(())
        }
        Command::Auth { command: AuthCommand::AuthorizeUrl } => {
            let (url, state) = auth::create_authorization_request(&config_path, config.clone())?;
            println!("Open this URL in your browser:\n{url}\n");
            println!("OAuth state (compare with the state query parameter after redirect):\n{state}");
            Ok(())
        }
        Command::Auth { command: AuthCommand::ExchangeCode(args) } => {
            auth::exchange_code(&config_path, config, &args.code, &args.state).await?;
            println!("Webex authorization completed. Tokens were saved locally.");
            Ok(())
        }
        Command::Auth { command: AuthCommand::Refresh } => {
            auth::refresh_token(&config_path, config).await?;
            println!("Webex access token refreshed and saved.");
            Ok(())
        }
        Command::Auth { command: AuthCommand::Status } => {
            if config.has_access_token() {
                println!("Authorized; access token expiry (Unix UTC): {}.", config.access_token_expires_at.unwrap_or_default());
            } else if config.refresh_token.is_some() {
                println!("Refresh token is present; an access token refresh is required.");
            } else {
                println!("Not authorized. Run wxcc_handler auth authorize-url.");
            }
            Ok(())
        }
        Command::Data { command: DataCommand::Collect } => {
            let output_path = resolve_output_path(cli.output.as_deref(), &exe_dir);
            let rows = data::collect_to_csv(&config_path, config, &output_path).await?;
            println!("Data collection completed: {rows} rows exported to {}", output_path.display());
            Ok(())
        }
    };
    if let Err(err) = result {
        error!("Command failed: {err:#}");
        return Err(err);
    }
    info!("WXCC Handler command completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn runner_action_manifest_exposes_flat_actions_and_conditional_arguments() {
        let manifest = get_manifest();
        let action = manifest.arguments.iter().find(|arg| arg.name == "--action").unwrap();
        assert!(action.required);
        assert_eq!(action.options.as_ref().unwrap(), &vec![
            "config-init", "config-check", "auth-authorize-url", "auth-exchange-code",
            "auth-refresh", "auth-status", "data-collect"
        ]);
        for name in ["--code", "--state"] {
            let arg = manifest.arguments.iter().find(|arg| arg.name == name).unwrap();
            assert!(arg.required);
            assert_eq!(arg.depends_on.as_ref().unwrap().get("--action").unwrap(), &vec!["auth-exchange-code"]);
        }
        let output = manifest.arguments.iter().find(|arg| arg.name == "--output").unwrap();
        assert_eq!(output.depends_on.as_ref().unwrap().get("--action").unwrap(), &vec!["data-collect"]);
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn runner_action_cli_parses_and_maps_to_existing_operation() {
        let cli = Cli::try_parse_from(["wxcc_handler", "--action", "data-collect", "--output", "out.csv"]).unwrap();
        assert!(matches!(command_from_runner_action(cli.action.unwrap(), cli.code.as_deref(), cli.state.as_deref()).unwrap(), Command::Data { command: DataCommand::Collect }));
        assert_eq!(cli.output.as_deref(), Some(Path::new("out.csv")));
    }

    #[test]
    fn runner_oauth_exchange_requires_code_and_state() {
        let cli = Cli::try_parse_from(["wxcc_handler", "--action", "auth-exchange-code", "--code", "c", "--state", "s"]).unwrap();
        assert!(matches!(command_from_runner_action(&cli).unwrap(), Command::Auth { command: AuthCommand::ExchangeCode(_) }));
        assert!(Cli::try_parse_from(["wxcc_handler", "--action", "auth-exchange-code", "--code", "c"]).is_err());
    }

    #[test]
    fn help_exposes_runner_action_and_manual_subcommands() {
        let help = Cli::command().render_help().to_string();
        assert!(help.contains("--action"));
        assert!(help.contains("config"));
        assert!(help.contains("auth"));
        assert!(help.contains("data"));
    }

    #[test]
    fn relative_and_default_config_paths_use_executable_directory() {
        let base = Path::new("C:/apps/wxcc");
        assert_eq!(resolve_config_path(Some(Path::new("settings.json")), base), base.join("settings.json"));
        assert_eq!(resolve_config_path(None, base), base.join("wxcc_config.json"));
    }

    #[test]
    fn output_paths_default_to_executable_directory_and_preserve_absolute_paths() {
        let base = Path::new("C:/apps/wxcc");
        assert_eq!(resolve_output_path(None, base), base.join("wxcc_data.csv"));
        assert_eq!(resolve_output_path(Some(Path::new("reports/data.csv")), base), base.join("reports/data.csv"));
        let absolute = PathBuf::from("C:/exports/data.csv");
        assert_eq!(resolve_output_path(Some(&absolute), base), absolute);
    }

    #[test]
    fn absolute_config_path_is_preserved() {
        let path = if cfg!(windows) { PathBuf::from("C:/data/wxcc.json") } else { PathBuf::from("/data/wxcc.json") };
        assert_eq!(resolve_config_path(Some(&path), Path::new(".")), path);
    }
}
