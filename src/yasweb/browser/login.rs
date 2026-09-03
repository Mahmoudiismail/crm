use anyhow::Result;
use headless_chrome::Tab;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

use crate::yasweb::browser::debug;
use crate::yasweb::config::YaswebConfig;

/// Automates the login flow and waits for the dashboard to appear.
///
/// The function records HTML snapshots for successful navigation and login steps,
/// updating `step_num` after each snapshot. It returns an error when required
/// login elements cannot be found, input cannot be entered, the login button
/// cannot be clicked, a login error is displayed, or the dashboard does not
/// appear within the timeout.
///
/// # Parameters
///
/// * `active_report_name` — Identifies the report associated with saved HTML snapshots.
/// * `step_num` — Step counter used for naming snapshots; incremented after successful navigation and login.
///
/// # Examples
///
/// ```no_run,ignore
/// # use std::sync::Arc;
/// # let tab: Arc<Tab> = unimplemented!();
/// # let config: YaswebConfig = unimplemented!();
/// let mut step_num = 0;
/// execute_login(&tab, &config, "login", &mut step_num)?;
/// # Ok::<(), anyhow::Error>(())
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
        if let Ok(html) = tab.get_content() {
            error!("Page HTML at failure to find username:\n{}", html);
        }
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
                            if let Ok(html) = tab.get_content() {
                                error!("Page HTML at failure to type password:\n{}", html);
                            }
                            return Err(anyhow::anyhow!("Failed to type password"));
                        }
                        info!("Successfully typed password.");
                    }
                    Err(e) => {
                        error!("Failed to find password input: {:?}", e);
                        if let Ok(html) = tab.get_content() {
                            error!("Page HTML at failure to find password input:\n{}", html);
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
                    if let Err(e) = login_button.click() {
                        error!("Failed to click login button: {:?}", e);
                        if let Ok(html) = tab.get_content() {
                            error!("Page HTML after failed login click:\n{}", html);
                        }
                        return Err(anyhow::anyhow!("Failed to click login button"));
                    }
                    info!("Successfully clicked login button.");
                    if let Ok(html) = tab.get_content() {
                        info!("Page HTML after clicking login:\n{}", html);
                    }
                    debug::save_html_state(tab, active_report_name, *step_num, "After login");
                    *step_num += 1;
                }
                Err(e) => {
                    error!("Failed to find login button: {:?}", e);
                    if let Ok(html) = tab.get_content() {
                        error!("Page HTML at failure to find login button:\n{}", html);
                    }
                    return Err(anyhow::anyhow!("Failed to find login button"));
                }
            }

            info!("Waiting for dashboard to load or error message...");
            let mut login_success = false;
            for _ in 0..60 {
                if let Ok(err_alert) = tab.find_element(".alert-danger.fade.show") {
                    let msg = err_alert.get_inner_text().unwrap_or_default();
                    error!("Login failed: {}", msg.trim());
                    if let Ok(html) = tab.get_content() {
                        error!("Page HTML after failed login:\n{}", html);
                    }
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
                            "Found username on page ('{}') does not match expected '{}'",
                            inner_text, config.username
                        );
                    }
                    break;
                }
                std::thread::sleep(Duration::from_millis(500));
            }

            if !login_success {
                error!("Dashboard did not load within timeout, or login failed silently.");
                if let Ok(html) = tab.get_content() {
                    error!("Page HTML after wait timeout:\n{}", html);
                }
                return Err(anyhow::anyhow!("Dashboard wait timeout"));
            }

            Ok(())
        }
        Err(e) => {
            error!(
                "Failed to find username input, likely because page did not load: {:?}",
                e
            );
            if let Ok(html) = tab.get_content() {
                error!("Page HTML at failure to find username:\n{}", html);
            }
            Err(anyhow::anyhow!("Failed to find elements to login"))
        }
    }
}
