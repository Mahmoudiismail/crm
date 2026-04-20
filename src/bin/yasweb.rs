use anyhow::{Context, Result};
use headless_chrome::protocol::cdp::types::Event;
use headless_chrome::{Browser, LaunchOptions};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

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
            username: "3245".to_string(),
            password: Some("Soso@2350181".to_string()),
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
    let mut user_data_dir = std::env::current_exe().unwrap_or_default();
    user_data_dir.pop();
    user_data_dir.push("yasweb_chrome_data");

    let args = vec![
        std::ffi::OsStr::new("--ignore-certificate-errors"),
        std::ffi::OsStr::new("--start-maximized"),
    ];

    let launch_options = LaunchOptions::default_builder()
        .headless(config.headless)
        .sandbox(false)
        .idle_browser_timeout(std::time::Duration::from_secs(120))
        .user_data_dir(Some(user_data_dir))
        .args(args)
        .build()
        .unwrap();

    let browser = Browser::new(launch_options).context("Failed to launch browser")?;
    // Use the first initial tab instead of opening a new one
    let tab = {
        let tabs = browser.get_tabs().lock().unwrap();
        tabs.first().cloned()
    };
    let tab = match tab {
        Some(t) => t,
        None => browser.new_tab().context("Failed to open new tab")?,
    };

    // Enable network logging
    tab.enable_log()
        .context("Failed to enable network domain")?;

    // Add event listener for network
    let events = tab
        .add_event_listener(Arc::new(|event: &Event| match event {
            Event::NetworkRequestWillBeSent(req) => {
                info!(
                    "Request: {} {}",
                    req.params.request.method, req.params.request.url
                );
            }
            Event::NetworkResponseReceived(res) => {
                info!(
                    "Response: {} {} {}",
                    res.params.response.status,
                    res.params.response.url,
                    res.params.response.mime_type
                );
            }
            _ => {}
        }))
        .context("Failed to add event listener")?;

    info!("Navigating to {}", config.url);
    if let Err(e) = tab.navigate_to(&config.url) {
        error!("Navigate failed: {:?}", e);
        if let Ok(html) = tab.get_content() {
            error!("Page HTML after navigation failure:\n{}", html);
        }
        println!(
            "Warning: navigate to {} returned error, continuing anyway...",
            config.url
        );
    } else {
        info!("Successfully navigated to {}", config.url);
        if let Ok(html) = tab.get_content() {
            info!("Page HTML after navigation:\n{}", html);
        }
    }

    // Attempt to wait until navigated, ignore error if it timeouts but page loads
    let _ = tab.wait_until_navigated();

    // Give Angular more time to bootstrap and render the initial DOM
    std::thread::sleep(Duration::from_secs(10));

    info!("Waiting for username input...");
    let username_selector = "input[formcontrolname='username'], #mat-input-0";

    // Custom wait loop to wait longer than default timeout
    let mut username_found = false;
    for _ in 0..6 {
        // 6 * 5 = 30 seconds max wait
        if let Ok(_) = tab.wait_for_element(username_selector) {
            username_found = true;
            break;
        }
        std::thread::sleep(Duration::from_secs(5));
    }

    if !username_found {
        error!("Failed to find username input after extended wait.");
        if let Ok(html) = tab.get_content() {
            error!("Page HTML at failure to find username:\n{}", html);
        }

        std::thread::sleep(Duration::from_secs(60));
        return Err(anyhow::anyhow!("Failed to find elements to login"));
    }

    match tab.wait_for_element(username_selector) {
        Ok(user_input) => {
            info!("Typing username...");
            if let Err(e) = user_input.type_into(&config.username) {
                error!("Failed to type username: {:?}", e);
                if let Ok(html) = tab.get_content() {
                    error!("Page HTML after failed username typing:\n{}", html);
                }

                std::thread::sleep(Duration::from_secs(60));
                return Err(anyhow::anyhow!("Failed to type username"));
            }
            info!("Successfully typed username.");
            if let Ok(html) = tab.get_content() {
                info!("Page HTML after typing username:\n{}", html);
            }

            // Wait a brief moment to ensure page loads data after username
            std::thread::sleep(Duration::from_secs(2));

            if let Some(password) = &config.password {
                info!("Waiting for password input...");
                let password_selector = "input[formcontrolname='password'], #passFocus";
                match tab.wait_for_element(password_selector) {
                    Ok(pass_input) => {
                        info!("Typing password...");
                        if let Err(e) = pass_input.type_into(password) {
                            error!("Failed to type password: {:?}", e);
                            if let Ok(html) = tab.get_content() {
                                error!("Page HTML after failed password typing:\n{}", html);
                            }

                            std::thread::sleep(Duration::from_secs(60));
                            return Err(anyhow::anyhow!("Failed to type password"));
                        }
                        info!("Successfully typed password.");
                        if let Ok(html) = tab.get_content() {
                            info!("Page HTML after typing password:\n{}", html);
                        }
                    }
                    Err(e) => {
                        error!("Failed to find password input: {:?}", e);
                        if let Ok(html) = tab.get_content() {
                            error!("Page HTML at failure to find password input:\n{}", html);
                        }

                        std::thread::sleep(Duration::from_secs(60));
                        return Err(anyhow::anyhow!("Failed to find password input"));
                    }
                }
            }

            info!("Waiting for login button...");
            let button_selector = "button#submitFocus, button.pmry";
            match tab.wait_for_element(button_selector) {
                Ok(login_button) => {
                    info!("Clicking login button...");
                    if let Err(e) = login_button.click() {
                        error!("Failed to click login button: {:?}", e);
                        if let Ok(html) = tab.get_content() {
                            error!("Page HTML after failed login click:\n{}", html);
                        }

                        std::thread::sleep(Duration::from_secs(60));
                        return Err(anyhow::anyhow!("Failed to click login button"));
                    }
                    info!("Successfully clicked login button.");
                    if let Ok(html) = tab.get_content() {
                        info!("Page HTML after clicking login:\n{}", html);
                    }
                }
                Err(e) => {
                    error!("Failed to find login button: {:?}", e);
                    if let Ok(html) = tab.get_content() {
                        error!("Page HTML at failure to find login button:\n{}", html);
                    }

                    std::thread::sleep(Duration::from_secs(60));
                    return Err(anyhow::anyhow!("Failed to find login button"));
                }
            }

            info!("Waiting for dashboard to load or error message...");
            let mut login_success = false;
            for _ in 0..15 {
                // Wait up to 30 seconds (15 * 2s)
                if let Ok(err_alert) = tab.find_element(".alert-danger.fade.show") {
                    let msg = err_alert.get_inner_text().unwrap_or_default();
                    error!("Login failed: {}", msg.trim());
                    if let Ok(html) = tab.get_content() {
                        error!("Page HTML after failed login:\n{}", html);
                    }

                    std::thread::sleep(Duration::from_secs(60));
                    return Err(anyhow::anyhow!("Login failed: {}", msg.trim()));
                }

                if let Ok(usr_id_element) = tab.find_element("span.usr-id") {
                    login_success = true;
                    let inner_text = usr_id_element.get_inner_text().unwrap_or_default();
                    if inner_text.contains(&config.username) {
                        info!(
                            "Successfully verified username {} on the page.",
                            config.username
                        );
                        println!("Verified username {} on the page.", config.username);
                    } else {
                        error!(
                            "Username mismatch! Found '{}', expected '{}'",
                            inner_text, config.username
                        );
                        if let Ok(html) = tab.get_content() {
                            error!("Page HTML at username verification mismatch:\n{}", html);
                        }
                    }
                    break;
                }
                std::thread::sleep(Duration::from_secs(2));
            }

            if !login_success {
                error!("Failed to reach dashboard or find error message");
                if let Ok(html) = tab.get_content() {
                    error!("Page HTML at dashboard timeout:\n{}", html);
                }

                std::thread::sleep(Duration::from_secs(60));
                return Err(anyhow::anyhow!("Dashboard timeout"));
            }

            info!("Waiting for menu to fully render...");
            std::thread::sleep(Duration::from_secs(2)); // Short delay for Angular to stabilize
            let mut menu_found = false;
            for _ in 0..10 {
                // Wait up to 20 seconds (10 * 2s)
                if let Ok(_) = tab.find_element("#menuPinnedBtn") {
                    menu_found = true;
                    break;
                }
                std::thread::sleep(Duration::from_secs(2));
            }
            if !menu_found {
                error!("Menu #menuPinnedBtn not found after wait.");
                if let Ok(html) = tab.get_content() {
                    error!("Page HTML at menu wait timeout:\n{}", html);
                }

                std::thread::sleep(Duration::from_secs(60));
                return Err(anyhow::anyhow!("Menu wait timeout"));
            }

            info!("Opening menu via #menuPinnedBtn...");
            match tab.wait_for_element("#menuPinnedBtn") {
                Ok(menu_btn) => {
                    if let Err(e) = menu_btn.click() {
                        error!("Failed to click #menuPinnedBtn: {:?}", e);
                        if let Ok(html) = tab.get_content() {
                            error!("Page HTML after failed #menuPinnedBtn click:\n{}", html);
                        }
                    } else {
                        info!("Successfully clicked #menuPinnedBtn.");
                        if let Ok(html) = tab.get_content() {
                            info!("Page HTML after clicking #menuPinnedBtn:\n{}", html);
                        }

                        // Wait for MIS module to appear
                        info!("Waiting for MIS module to appear in menu...");
                        std::thread::sleep(Duration::from_secs(2)); // Short delay
                        let mis_selector = ".menu-grid-item.misManagement";
                        let mut mis_found = false;
                        for _ in 0..10 {
                            // Wait up to 20 seconds (10 * 2s)
                            if let Ok(_) = tab.find_element(mis_selector) {
                                mis_found = true;
                                break;
                            }
                            std::thread::sleep(Duration::from_secs(2));
                        }
                        if !mis_found {
                            error!("MIS module not found after wait.");
                            if let Ok(html) = tab.get_content() {
                                error!("Page HTML at MIS module wait timeout:\n{}", html);
                            }

                            std::thread::sleep(Duration::from_secs(60));
                            return Err(anyhow::anyhow!("MIS module wait timeout"));
                        }
                        match tab.wait_for_element(mis_selector) {
                            Ok(mis_module) => {
                                info!("Clicking on MIS module...");
                                if let Err(e) = mis_module.click() {
                                    error!("Failed to click MIS module: {:?}", e);
                                    if let Ok(html) = tab.get_content() {
                                        error!(
                                            "Page HTML after failed MIS module click:\n{}",
                                            html
                                        );
                                    }
                                } else {
                                    info!("Clicked MIS successfully.");
                                    println!("Clicked MIS successfully.");
                                    if let Ok(html) = tab.get_content() {
                                        info!("Page HTML after clicking MIS module:\n{}", html);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to find MIS module: {:?}", e);
                                if let Ok(html) = tab.get_content() {
                                    error!("Page HTML at failure to find MIS module:\n{}", html);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to find #menuPinnedBtn: {:?}", e);
                    if let Ok(html) = tab.get_content() {
                        error!("Page HTML at failure to find #menuPinnedBtn:\n{}", html);
                    }
                }
            }
        }
        Err(e) => {
            error!(
                "Failed to find username input, likely because page did not load: {:?}",
                e
            );
            if let Ok(html) = tab.get_content() {
                error!("Page HTML at failure to find username:\n{}", html);
            }

            std::thread::sleep(Duration::from_secs(60));
            return Err(anyhow::anyhow!("Failed to find elements to login"));
        }
    }

    // Remove listener before exit
    tab.remove_event_listener(&events)
        .context("Failed to remove listener")?;

    std::thread::sleep(Duration::from_secs(60));
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
    })
    .await?;

    Ok(())
}
