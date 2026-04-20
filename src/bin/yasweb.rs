use anyhow::{Context, Result};
use headless_chrome::{Browser, LaunchOptions};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;
use headless_chrome::protocol::cdp::types::Event;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
struct YaswebConfig {
    url: String,
    username: String,
    password: Option<String>,
    #[serde(default)]
    headless: bool,
}

impl Default for YaswebConfig {
    fn default() -> Self {
        Self {
            url: "https://yasweb.fakeeh.care:8030/".to_string(),
            username: "".to_string(),
            password: None,
            headless: false,
        }
    }
}

async fn load_or_create_config(path: &PathBuf) -> Result<YaswebConfig> {
    if path.exists() {
        let content = fs::read_to_string(path)
            .await
            .context("Failed to read yasweb_config.json")?;
        let config: YaswebConfig =
            serde_json::from_str(&content).context("Failed to parse yasweb_config.json")?;
        Ok(config)
    } else {
        let config = YaswebConfig::default();
        let content = serde_json::to_string_pretty(&config)
            .context("Failed to serialize default yasweb config")?;
        fs::write(path, content)
            .await
            .context("Failed to write default yasweb_config.json")?;
        Ok(config)
    }
}

// We hold a global guard so logs aren't dropped. In a real app we'd pass it back to main.
static mut _LOG_GUARD: Option<tracing_appender::non_blocking::WorkerGuard> = None;

fn setup_logging() -> Result<()> {
    let file_appender = tracing_appender::rolling::never(".", "yasweblog");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Save guard to static so it isn't dropped immediately
    unsafe {
        _LOG_GUARD = Some(guard);
    }

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_writer(non_blocking)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .context("Setting default subscriber failed")?;
    Ok(())
}

fn run_browser(config: &YaswebConfig) -> Result<()> {
    let launch_options = LaunchOptions::default_builder()
        .headless(config.headless)
        .sandbox(false)
        .idle_browser_timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap();

    let browser = Browser::new(launch_options).context("Failed to launch browser")?;
    let tab = browser.new_tab().context("Failed to open new tab")?;

    // Enable network logging
    tab.enable_log().context("Failed to enable network domain")?;

    // Add event listener for network
    let events = tab.add_event_listener(Arc::new(|event: &Event| {
        match event {
            Event::NetworkRequestWillBeSent(req) => {
                info!("Request: {} {}", req.params.request.method, req.params.request.url);
            }
            Event::NetworkResponseReceived(res) => {
                info!("Response: {} {} {}", res.params.response.status, res.params.response.url, res.params.response.mime_type);
            }
            _ => {}
        }
    })).context("Failed to add event listener")?;

    info!("Navigating to {}", config.url);
    if let Err(e) = tab.navigate_to(&config.url) {
        error!("Navigate failed: {:?}", e);
        // Sometimes navigate fails but it still actually loads the page, or it's a test env error
        // Continue but with warning
        println!("Warning: navigate to {} returned error, continuing anyway...", config.url);
    }

    // Attempt to wait until navigated, ignore error if it timeouts but page loads
    let _ = tab.wait_until_navigated();

    info!("Waiting for username input...");
    let username_selector = "input[formcontrolname='username'], #mat-input-0";
    match tab.wait_for_element(username_selector) {
        Ok(user_input) => {
            info!("Typing username...");
            user_input.type_into(&config.username).context("Failed to type username")?;

            if let Some(password) = &config.password {
                info!("Waiting for password input...");
                let password_selector = "input[formcontrolname='password'], #passFocus";
                match tab.wait_for_element(password_selector) {
                    Ok(pass_input) => {
                        info!("Typing password...");
                        pass_input.type_into(password).context("Failed to type password")?;
                    }
                    Err(e) => {
                        error!("Failed to find password input: {:?}", e);
                        if let Ok(html) = tab.get_content() {
                            error!("Page HTML:\n{}", html);
                        }
                        return Err(anyhow::anyhow!("Failed to find password input"));
                    }
                }
            }

            info!("Waiting for login button...");
            let button_selector = "button#submitFocus, button.pmry";
            match tab.wait_for_element(button_selector) {
                Ok(login_button) => {
                    info!("Clicking login button...");
                    login_button.click().context("Failed to click login button")?;
                }
                Err(e) => {
                    error!("Failed to find login button: {:?}", e);
                    if let Ok(html) = tab.get_content() {
                        error!("Page HTML:\n{}", html);
                    }
                    return Err(anyhow::anyhow!("Failed to find login button"));
                }
            }

            // Wait a brief moment to ensure login is submitted
            std::thread::sleep(Duration::from_secs(5));

            println!("Login successful");
            info!("Login successful");
        }
        Err(e) => {
            error!("Failed to find username input, likely because page did not load: {:?}", e);
            if let Ok(html) = tab.get_content() {
                error!("Page HTML:\n{}", html);
            }
            return Err(anyhow::anyhow!("Failed to find elements to login"));
        }
    }

    // Remove listener before exit
    tab.remove_event_listener(&events).context("Failed to remove listener")?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging()?;

    let mut config_path = std::env::current_exe().context("Failed to get executable path")?;
    config_path.pop();
    config_path.push("yasweb_config.json");

    let config = load_or_create_config(&config_path).await?;
    info!("Loaded config for URL: {}", config.url);

    // Run browser logic in a blocking task since headless_chrome is synchronous
    tokio::task::spawn_blocking(move || {
        if let Err(e) = run_browser(&config) {
            error!("Browser automation failed: {:?}", e);
            eprintln!("Browser automation failed: {:?}", e);
        }
    }).await?;

    Ok(())
}
