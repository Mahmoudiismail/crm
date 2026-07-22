use anyhow::{Context, Result};
use clap::Parser;
use crm_tool::manifest::{AppArg, AppManifest, ArgType};
use crm_tool::utils::{intercept_manifest, load_or_create_config, parse_log_level, setup_logging_with_levels};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "wcxx", about = "Webex Contact Center Fetcher")]
struct WcxxCliOptions {
    #[arg(long, default_value = "wcxx_config.json")]
    config: String,
    #[arg(long, hide = true)]
    manifest: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    #[serde(default = "default_base_url")]
    base_url: String,
    token: String,
    #[serde(default)]
    org_id: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    client_id: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    client_secret: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    refresh_token: Option<String>,
    #[serde(default = "default_stdout_log_level")]
    log_stdout_level: String,
    #[serde(default = "default_file_log_level")]
    log_file_level: String,
}

fn default_stdout_log_level() -> String {
    "DEBUG".to_string()
}

fn default_file_log_level() -> String {
    "TRACE".to_string()
}

fn default_base_url() -> String {
    "https://webexapis.com/v1".to_string()
}

fn get_manifest() -> AppManifest {
    AppManifest {
        name: "Webex Contact Center Fetcher".to_string(),
        description: "Fetches data from the Webex Contact Center API.".to_string(),
        arguments: vec![AppArg {
            name: "--config".to_string(),
            arg_type: ArgType::String,
            required: false,
            default_value: None,
            options: None,
            depends_on: None,
            autofill: None,
        }],
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    intercept_manifest(get_manifest());

    let options = WcxxCliOptions::parse();
    let config_path = PathBuf::from(options.config);

    let default_config = Config {
        base_url: "https://webexapis.com/v1".to_string(),
        token: "YOUR_BEARER_TOKEN_HERE".to_string(),
        org_id: Some("".to_string()),
        client_id: Some("".to_string()),
        client_secret: Some("".to_string()),
        refresh_token: Some("".to_string()),
        log_stdout_level: "DEBUG".to_string(),
        log_file_level: "TRACE".to_string(),
    };

    if !config_path.exists() {
        load_or_create_config(&config_path, &default_config)?;
        println!(
            "Created default configuration file at {:?}. Please edit it and re-run.",
            config_path
        );
        return Ok(());
    }

    let config: Config = load_or_create_config(&config_path, &default_config)?;

    if config.token == "YOUR_BEARER_TOKEN_HERE" || config.token.trim().is_empty() {
        anyhow::bail!("Please update {:?} with a valid token.", config_path);
    }

    let _guard = setup_logging_with_levels(
        "wcxx",
        parse_log_level(&config.log_stdout_level),
        parse_log_level(&config.log_file_level)
    )?;

    info!("Starting wcxx tool");

    let client = Client::new();

    let endpoints = vec![
        ("calendars", "/organization/calendars"),
        ("agents", "/organization/agents"),
        ("teams", "/organization/teams"),
        ("queues", "/organization/queues"),
        ("skills", "/organization/skills"),
    ];

    let mut results = serde_json::Map::new();

    for (name, path) in endpoints {
        let url = format!("{}{}", config.base_url.trim_end_matches('/'), path);
        info!("Fetching {}", url);
        let mut request = client.get(&url).bearer_auth(&config.token);

        // Some APIs might require orgId as query param if provided
        if let Some(org_id) = &config.org_id {
            if !org_id.is_empty() {
                request = request.query(&[("orgId", org_id)]);
            }
        }

        match request.send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    let text = resp.text().await?;
                    let json: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
                    results.insert(name.to_string(), json);
                    info!("Successfully fetched {}", name);
                } else {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    error!("Failed to fetch {}: {} - {}", name, status, text);
                    results.insert(
                        name.to_string(),
                        serde_json::json!({
                            "error": format!("Status: {}", status),
                            "details": text
                        }),
                    );
                }
            }
            Err(e) => {
                error!("Network error fetching {}: {:?}", name, e);
                results.insert(
                    name.to_string(),
                    serde_json::json!({
                        "error": "Network error",
                        "details": e.to_string()
                    }),
                );
            }
        }
    }

    let final_json = Value::Object(results);
    let json_string = serde_json::to_string_pretty(&final_json)?;

    // Write to a temporary HTML file so the browser renders it nicely (or just displays the JSON text)
    let html_content = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>WCXX Data</title>
    <style>
        body {{ font-family: monospace; background: #f4f4f4; padding: 20px; }}
        pre {{ background: #fff; padding: 15px; border: 1px solid #ddd; overflow: auto; }}
    </style>
</head>
<body>
    <h2>WCXX Data Export</h2>
    <pre id="json"></pre>
    <script>
        const data = {};
        document.getElementById('json').textContent = JSON.stringify(data, null, 4);
    </script>
</body>
</html>"#,
        json_string
    );

    let output_file = env::temp_dir().join("wcxx_output.html");
    fs::write(&output_file, html_content).context("Failed to write output HTML file")?;

    info!("Wrote results to {:?}", output_file);
    println!(
        "Data successfully retrieved! Opening in browser: {:?}",
        output_file
    );

    if let Err(e) = open::that(&output_file) {
        error!("Failed to open browser: {:?}", e);
        println!(
            "Failed to open browser: {:?}. You can manually open {:?}",
            e, output_file
        );
    }

    info!("Finished wcxx tool");
    Ok(())
}
