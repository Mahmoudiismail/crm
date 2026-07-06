use anyhow::{Context, Result};
use crm_tool::manifest::{AppArg, AppManifest, ArgType};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;
use tracing::{error, info, Level};

#[derive(Debug, Deserialize)]
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
}

fn default_base_url() -> String {
    "https://webexapis.com/v1".to_string()
}

fn print_manifest() {
    let manifest = AppManifest {
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
    };
    if let Ok(json) = serde_json::to_string(&manifest) {
        println!("{}", json);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Intercept --manifest before anything else
    for arg in env::args().skip(1) {
        if arg == "--manifest" {
            print_manifest();
            std::process::exit(0);
        }
    }

    // Read command line arguments to find the config file path (default: wcxx_config.json)
    let args: Vec<String> = env::args().collect();
    let mut config_path = PathBuf::from("wcxx_config.json");

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--config" && i + 1 < args.len() {
            config_path = PathBuf::from(&args[i + 1]);
            break;
        }
        i += 1;
    }

    if !config_path.exists() {
        // Create a default config file if it doesn't exist
        let default_config = serde_json::json!({
            "base_url": "https://webexapis.com/v1",
            "token": "YOUR_BEARER_TOKEN_HERE",
            "org_id": "",
            "client_id": "",
            "client_secret": "",
            "refresh_token": ""
        });
        fs::write(&config_path, serde_json::to_string_pretty(&default_config)?)
            .context("Failed to write default wcxx_config.json")?;
        println!(
            "Created default configuration file at {:?}. Please edit it and re-run.",
            config_path
        );
        return Ok(());
    }

    let config_content = fs::read_to_string(&config_path)
        .context(format!("Failed to read config file at {:?}", config_path))?;
    let config: Config = serde_json::from_str(&config_content)
        .context("Failed to parse config file. Please ensure it contains 'base_url' and 'token'")?;

    if config.token == "YOUR_BEARER_TOKEN_HERE" || config.token.trim().is_empty() {
        anyhow::bail!("Please update {:?} with a valid token.", config_path);
    }

    // Set up logging to wcxx.log
    let file_appender = tracing_appender::rolling::never(".", "wcxx.log");
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_writer(file_appender)
        .init();

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
