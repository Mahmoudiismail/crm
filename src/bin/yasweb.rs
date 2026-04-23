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
    #[serde(default)]
    report_type: String,
    #[serde(default)]
    report_name: String,
}

impl Default for YaswebConfig {
    fn default() -> Self {
        Self {
            url: "https://yasweb.fakeeh.care:8030/".to_string(),
            username: "3245".to_string(),
            password: Some("Soso@2350181".to_string()),
            headless: false,
            report_type: "".to_string(),
            report_name: "".to_string(),
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

        let mut blank_tab = None;
        for t in tabs.iter() {
            if t.get_url().contains("about:blank") {
                blank_tab = Some(t.clone());
                break;
            }
        }

        if blank_tab.is_none() {
            // Log all tab URLs if about:blank not found
            let urls: Vec<String> = tabs.iter().map(|t| t.get_url()).collect();
            info!("Warning: No about:blank tab found. Open tabs: {:?}", urls);

            // Fallback to first tab
            blank_tab = tabs.first().cloned();
        }

        blank_tab
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
        println!(
            "Warning: navigate to {} returned error, continuing anyway...",
            config.url
        );
    } else {
        info!("Successfully navigated to {}", config.url);
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
                    error!("Page HTML at failure to type username:\n{}", html);
                }
                std::thread::sleep(Duration::from_secs(60));
                return Err(anyhow::anyhow!("Failed to type username"));
            }
            info!("Successfully typed username.");

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
                                error!("Page HTML at failure to type password:\n{}", html);
                            }
                            std::thread::sleep(Duration::from_secs(60));
                            return Err(anyhow::anyhow!("Failed to type password"));
                        }
                        info!("Successfully typed password.");
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
            let js_click_menu = r#"
                (function() {
                    try {
                        var innerBtn = document.querySelector('#menuPinnedBtn > div.icon.font-icon.mod-triger > i.bi.bi-plus.second');
                        if (innerBtn) {
                            innerBtn.click();
                            return "CLICKED";
                        }
                        var btn = document.querySelector('#menuPinnedBtn');
                        if (btn) { btn.click(); return "CLICKED"; }
                        return "NOT_FOUND";
                    } catch (e) {
                        return "ERROR: " + e.message;
                    }
                })();
            "#;

            let mut menu_clicked = false;
            if let Ok(eval_result) = tab.evaluate(js_click_menu, true) {
                if let Some(val) = eval_result.value {
                    if let Some(val_str) = val.as_str() {
                        if val_str == "CLICKED" {
                            info!("Successfully clicked #menuPinnedBtn via JS.");
                            menu_clicked = true;
                        } else {
                            error!("Failed to click #menuPinnedBtn via JS: {}", val_str);
                        }
                    }
                }
            }

            if !menu_clicked {
                // Fallback to native click
                match tab.wait_for_element("#menuPinnedBtn") {
                    Ok(menu_btn) => {
                        if let Err(e) = menu_btn.click() {
                            error!("Failed to click #menuPinnedBtn: {:?}", e);
                            if let Ok(html) = tab.get_content() {
                                error!(
                                    "Page HTML after failed #menuPinnedBtn click:
{}",
                                    html
                                );
                            }
                        } else {
                            info!("Successfully clicked #menuPinnedBtn (fallback native).");
                            menu_clicked = true;
                        }
                    }
                    Err(e) => {
                        error!("Failed to find #menuPinnedBtn for fallback click: {:?}", e);
                        if let Ok(html) = tab.get_content() {
                            error!(
                                "Page HTML at failure to find #menuPinnedBtn:
{}",
                                html
                            );
                        }
                    }
                }
            }

            if menu_clicked {
                // Wait for menuPinnedBtn to have 'active' class
                info!("Waiting for #menuPinnedBtn to have 'active' class...");
                let mut is_active = false;
                let mut last_status = String::new();
                let check_active_js = r#"
                    (function() {
                        var btn = document.querySelector('#menuPinnedBtn');
                        if (!btn) return "NOT_FOUND";
                        if (btn.classList.contains("active")) {
                            return "ACTIVE";
                        }
                        return "CLASSES: " + btn.className;
                    })();
                "#;

                for _ in 0..10 {
                    if let Ok(eval_result) = tab.evaluate(check_active_js, true) {
                        if let Some(val) = eval_result.value {
                            if let Some(val_str) = val.as_str() {
                                if val_str == "ACTIVE" {
                                    is_active = true;
                                    break;
                                }
                                last_status = val_str.to_string();
                            }
                        }
                    }
                    std::thread::sleep(Duration::from_secs(2));
                }

                if !is_active {
                    error!("#menuPinnedBtn did not become active after wait. Last status: {}", last_status);
                    if let Ok(html) = tab.get_content() {
                        error!(
                            "Page HTML at menu active timeout:
{}",
                            html
                        );
                    }
                    std::thread::sleep(Duration::from_secs(60));
                    return Err(anyhow::anyhow!("Menu active state wait timeout"));
                }

                info!("Menu is active.");

                // Keep structure same to close out match
                // We fake the `match` structure to minimize changes required at the end of the block
            }

            // Re-creating the original match to not break the brace structure down below
            match Ok::<(), ()>(()) {
                Ok(_) => {
                    if !menu_clicked {
                        error!("Menu was not clicked");
                    } else {
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
                                    info!("Clicked MIS successfully. Proceeding to report selection and search.");

                                    // Wait for MIS module to stabilize
                                    std::thread::sleep(Duration::from_secs(5));

                                    // If a report_type is provided, find and click the corresponding radio button
                                    // and then type the report_name in the search box.
                                    if !config.report_type.is_empty() {
                                        info!("Selecting report type: {} and searching for: {}", config.report_type, config.report_name);

                                        // The report UI is inside an iframe. We evaluate JS to find the radio button and search input.
                                        let js_eval = format!(
                                            r#"
                                            (function() {{
                                                var iframes = document.querySelectorAll("iframe");
                                                var targetType = "{}";
                                                var targetName = "{}";

                                                for (var i = 0; i < iframes.length; i++) {{
                                                    try {{
                                                        var doc = iframes[i].contentWindow.document;

                                                        // 1. Find and click the radio button for report type
                                                        var radioXpath = "//*[contains(text(), '" + targetType + "')]/ancestor-or-self::mat-radio-button";
                                                        var radioResult = doc.evaluate(radioXpath, doc, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                                                        var radioNode = radioResult.singleNodeValue;

                                                        if (radioNode) {{
                                                            radioNode.click();

                                                            // 2. Find the search input #mat-input-0 and type the report name
                                                            // We might need a small delay for the radio selection to register,
                                                            // but let's try finding the input immediately first.
                                                            var searchInput = doc.querySelector("#mat-input-0");
                                                            if (searchInput) {{
                                                                searchInput.value = targetName;
                                                                // Trigger Angular input event
                                                                searchInput.dispatchEvent(new Event('input', {{ bubbles: true }}));

                                                                // 3. Simulate Enter key press
                                                                var enterEvent = new KeyboardEvent('keydown', {{
                                                                    key: 'Enter',
                                                                    code: 'Enter',
                                                                    keyCode: 13,
                                                                    which: 13,
                                                                    bubbles: true
                                                                }});
                                                                searchInput.dispatchEvent(enterEvent);

                                                                return "SUCCESS: Clicked " + targetType + " and searched " + targetName;
                                                            }} else {{
                                                                return "RADIO_CLICKED_BUT_SEARCH_NOT_FOUND";
                                                            }}
                                                        }}
                                                    }} catch (e) {{
                                                        // Ignore cross-origin frame errors or other exceptions
                                                    }}
                                                }}
                                                return "NOT_FOUND_IN_ANY_IFRAME";
                                            }})();
                                        "#,
                                            config.report_type,
                                            config.report_name
                                        );

                                        let mut action_success = false;
                                        for _ in 0..15 {
                                            if let Ok(eval_result) = tab.evaluate(&js_eval, true) {
                                                if let Some(val) = eval_result.value {
                                                    if let Some(val_str) = val.as_str() {
                                                        if val_str.starts_with("SUCCESS") {
                                                            info!("Automation result: {}", val_str);
                                                            action_success = true;
                                                            break;
                                                        } else if val_str == "RADIO_CLICKED_BUT_SEARCH_NOT_FOUND" {
                                                            info!("Radio clicked, waiting for search box...");
                                                        }
                                                    }
                                                }
                                            }
                                            std::thread::sleep(Duration::from_secs(2));
                                        }

                                        if !action_success {
                                            error!("Failed to complete report selection or search inside iframe.");
                                            if let Ok(html) = tab.get_content() {
                                                error!("Main Page HTML at failure:\n{}", html);
                                            }
                                        } else {
                                            // Wait a bit to let the search results load
                                            std::thread::sleep(Duration::from_secs(5));
                                        }
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

    let mut config = load_or_create_config(&config_path).await?;

    // Parse CLI arguments to override report_type and report_name
    // Usage: yasweb [--type "Report Type"] [--name "Report Name"]
    let args: Vec<String> = std::env::args().collect();
    info!("CLI Arguments: {:?}", args);
    let mut config_updated = false;

        let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if let Some((key, value)) = arg.split_once('=') {
            match key {
                "--type" | "-t" => {
                    config.report_type = value.trim_matches(|c| c == '"' || c == '\'').to_string();
                    config_updated = true;
                }
                "--name" | "-n" => {
                    config.report_name = value.trim_matches(|c| c == '"' || c == '\'').to_string();
                    config_updated = true;
                }
                _ => {
                    if key.starts_with('-') {
                        eprintln!("Warning: Unknown argument: {}", arg);
                    }
                }
            }
        } else {
            match arg.as_str() {
                "--type" | "-t" => {
                    if i + 1 < args.len() {
                        config.report_type = args[i + 1].trim_matches(|c| c == '"' || c == '\'').to_string();
                        config_updated = true;
                        i += 1;
                    } else {
                        eprintln!("Error: {} requires a value", arg);
                        std::process::exit(1);
                    }
                }
                "--name" | "-n" => {
                    if i + 1 < args.len() {
                        config.report_name = args[i + 1].trim_matches(|c| c == '"' || c == '\'').to_string();
                        config_updated = true;
                        i += 1;
                    } else {
                        eprintln!("Error: {} requires a value", arg);
                        std::process::exit(1);
                    }
                }
                "--help" | "-h" => {
                    println!("Usage: yasweb [--type=\"Report Type\"] [--name=\"Report Name\"] [--headless]");
                    println!("  --type, -t   The type of report to select (e.g., \"Report Manager\" or \"Standard Report\")");
                    println!("  --name, -n   The name of the report (required if type is provided)");
                    println!("  --headless   Run browser in headless mode");
                    std::process::exit(0);
                }
                "--headless" => {
                    config.headless = true;
                    config_updated = true;
                }
                _ => {
                    if arg.starts_with('-') {
                        eprintln!("Warning: Unknown argument: {}", arg);
                    }
                }
            }
        }
        i += 1;
    } else {
                        eprintln!("Error: {} requires a value", arg);
                        std::process::exit(1);
                    }
                }
                "--name" | "-n" => {
                    if i + 1 < args.len() {
                        config.report_name = args[i + 1].trim_matches(|c| c == '"' || c == '\'').to_string();
                        config_updated = true;
                        i += 1;
                    } else {
                        eprintln!("Error: {} requires a value", arg);
                        std::process::exit(1);
                    }
                }
                "--help" | "-h" => {
                    println!("Usage: yasweb [--type=\"Report Type\"] [--name=\"Report Name\"] [--headless]");
                    println!("  --type, -t   The type of report to select (e.g., \"Report Manager\" or \"Standard Report\")");
                    println!("  --name, -n   The name of the report (required if type is provided)");
                    println!("  --headless   Run browser in headless mode");
                    std::process::exit(0);
                }
                "--headless" => {
                    config.headless = true;
                    config_updated = true;
                }
                _ => {
                    if arg.starts_with('-') {
                        eprintln!("Warning: Unknown argument: {}", arg);
                    }
                }
            }
        }
        i += 1;
    }

    // Ensure the constraint: if report_type is provided, report_name must also be provided.
    if !config.report_type.is_empty() && config.report_name.is_empty() {
        let err_msg = "Validation failed: report_type is provided but report_name is missing.";
        error!("{}", err_msg);
        eprintln!("{}", err_msg);
        std::process::exit(1);
    }

    if config_updated {
        info!("Updating yasweb_config.json with CLI report parameters...");
        let content = serde_json::to_string_pretty(&config)
            .context("Failed to serialize updated yasweb config")?;
        fs::write(&config_path, content)
            .await
            .context("Failed to write updated yasweb_config.json")?;
    }

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
