use anyhow::Result;
use headless_chrome::Tab;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

use crate::yasweb::browser::debug;
use crate::yasweb::config::YaswebConfig;

/// Automates the login flow and waits for the dashboard to appear.
///
/// Optional HTML diagnostics are captured only when
/// `CRM_YASWEB_CAPTURE_DIAGNOSTICS=1` is set. It returns an error when required
/// login elements cannot be found, input cannot be entered, the login button
/// cannot be clicked, a login error is displayed, or the dashboard does not
/// appear within the timeout.
///
/// # Parameters
///
/// * `active_report_name` — Identifies the report associated with optional diagnostic snapshots.
/// * `step_num` — Step counter used for naming optional snapshots.
///
/// # Examples
///
/// ```no_run
/// use std::sync::Arc;
/// use headless_chrome::Tab;
/// use crm_tool::yasweb::config::YaswebConfig;
/// use crm_tool::yasweb::browser::login::execute_login;
///
/// # fn example() -> anyhow::Result<()> {
/// # let tab: Arc<Tab> = unimplemented!();
/// # let config: YaswebConfig = unimplemented!();
/// let mut step_num = 0;
/// execute_login(&tab, &config, "login", &mut step_num)?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if login cannot be completed or the dashboard does not
/// appear within the timeout.
pub fn execute_login(
    tab: &Arc<Tab>,
    config: &YaswebConfig,
    active_report_name: &str,
    step_num: &mut u32,
) -> Result<()> {
    info!("Navigating to {}", config.url);
    if let Err(e) = tab.navigate_to(&config.url) {
        error!("Navigate failed: {:?}", e);
        println!(
            "Warning: navigate to {} returned error, continuing anyway...",
            config.url
        );
    } else {
        info!("Successfully navigated to {}", config.url);
        debug::save_html_state(tab, active_report_name, *step_num, "Main page load");
        *step_num += 1;
    }

    // Attempt to wait until navigated, ignore error if it timeouts but page loads
    let _ = tab.wait_until_navigated();

    info!("Waiting for username input...");
    let username_selector = "input[formcontrolname='username'], #mat-input-0";

    let mut username_found = false;
    if tab
        .wait_for_element_with_custom_timeout(username_selector, Duration::from_secs(30))
        .is_ok()
    {
        username_found = true;
    }

    if !username_found {
        error!("Failed to find username input after extended wait.");
        return Err(anyhow::anyhow!("Failed to find elements to login"));
    }

    match tab.wait_for_element(username_selector) {
        Ok(user_input) => {
            info!("Typing username...");
            if let Err(e) = user_input.type_into(&config.username) {
                error!("Failed to type username: {:?}", e);
                return Err(anyhow::anyhow!("Failed to type username"));
            }
            info!("Successfully typed username.");

            if let Some(password) = &config.password {
                info!("Waiting for password input...");
                let password_selector = "input[formcontrolname='password'], #passFocus";
                match tab.wait_for_element(password_selector) {
                    Ok(pass_input) => {
                        info!("Typing password...");
                        if let Err(e) = pass_input.type_into(password) {
                            error!("Failed to type password: {:?}", e);
                            return Err(anyhow::anyhow!("Failed to type password"));
                        }
                        info!("Successfully typed password.");
                    }
                    Err(e) => {
                        error!("Failed to find password input: {:?}", e);
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
                        return Err(anyhow::anyhow!("Failed to click login button"));
                    }
                    info!("Successfully clicked login button.");
                    debug::save_html_state(tab, active_report_name, *step_num, "After login");
                    *step_num += 1;
                }
                Err(e) => {
                    error!("Failed to find login button: {:?}", e);
                    return Err(anyhow::anyhow!("Failed to find login button"));
                }
            }

            info!("Waiting for dashboard to load or error message...");
            let mut login_success = false;
            for _ in 0..60 {
                if let Ok(err_alert) = tab.find_element(".alert-danger.fade.show") {
                    let msg = err_alert.get_inner_text().unwrap_or_default();
                    error!("Login failed: {}", msg.trim());
                    return Err(anyhow::anyhow!("Login failed: {}", msg.trim()));
                }

                if let Ok(usr_id_element) = tab.find_element("span.usr-id") {
                    login_success = true;
                    let inner_text = usr_id_element.get_inner_text().unwrap_or_default();
                    if inner_text.contains(&config.username) {
                        info!("Successfully verified configured username on the page.");
                    } else {
                        error!("The username displayed by the portal did not match the configured username.");
                    }
                    break;
                }
                std::thread::sleep(Duration::from_millis(500));
            }

            if !login_success {
                error!("Dashboard did not load within timeout, or login failed silently.");
                return Err(anyhow::anyhow!("Dashboard wait timeout"));
            }

            Ok(())
        }
        Err(e) => {
            error!(
                "Failed to find username input, likely because page did not load: {:?}",
                e
            );
            Err(anyhow::anyhow!("Failed to find elements to login"))
        }
    }
}
