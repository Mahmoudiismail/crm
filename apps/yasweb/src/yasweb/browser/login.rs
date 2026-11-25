use anyhow::Result;
use headless_chrome::protocol::cdp::Input;
use headless_chrome::Tab;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

use crate::yasweb::browser::debug;
use crate::yasweb::config::YaswebConfig;

/// Compare the portal's displayed account identifier with the configured username.
///
/// Trim surrounding whitespace and ignore case for textual usernames. Numeric IDs
/// are compared without leading zeros because the portal may pad them to a fixed width.
fn username_matches(displayed: &str, configured: &str) -> bool {
    let displayed = displayed.trim();
    let configured = configured.trim();
    if displayed.is_empty() || configured.is_empty() {
        return false;
    }

    if displayed.bytes().all(|byte| byte.is_ascii_digit())
        && configured.bytes().all(|byte| byte.is_ascii_digit())
    {
        let displayed_id = displayed.trim_start_matches('0');
        let configured_id = configured.trim_start_matches('0');
        let displayed_id = if displayed_id.is_empty() {
            "0"
        } else {
            displayed_id
        };
        let configured_id = if configured_id.is_empty() {
            "0"
        } else {
            configured_id
        };
        displayed_id == configured_id
    } else {
        displayed.eq_ignore_ascii_case(configured)
    }
}

fn build_login_input_focus_script(selector: &str) -> Result<String> {
    let selector_json = serde_json::to_string(selector)?;
    Ok(format!(
        r#"(() => {{
            const input = document.querySelector({selector_json});
            if (!input) return JSON.stringify({{status: "ERROR", reason: "not_found"}});
            if (input.disabled || input.readOnly || input.getClientRects().length === 0) {{
                return JSON.stringify({{status: "ERROR", reason: "not_interactable"}});
            }}
            input.focus();
            input.value = "";
            input.dispatchEvent(new Event("input", {{ bubbles: true }}));
            return JSON.stringify({{status: document.activeElement === input ? "SUCCESS" : "ERROR", reason: "focus_failed"}});
        }})()"#
    ))
}

fn build_login_input_verify_script(selector: &str, value: &str) -> Result<String> {
    let selector_json = serde_json::to_string(selector)?;
    let expected_json = serde_json::to_string(value)?;
    Ok(format!(
        r#"(() => {{
            const input = document.querySelector({selector_json});
            if (!input) return JSON.stringify({{status: "ERROR", reason: "input_disappeared"}});
            input.dispatchEvent(new Event("change", {{ bubbles: true }}));
            input.blur();
            return JSON.stringify({{status: input.value === {expected_json} ? "SUCCESS" : "ERROR", matches: input.value === {expected_json}, length: input.value.length}});
        }})()"#
    ))
}

/// Focuses an input and inserts text through Chrome's native input pipeline.
///
/// Using CDP Input.insertText emits the real input event consumed by Angular
/// ControlValueAccessor directives. The final DOM value is verified before the
/// login flow proceeds; values are never included in logs or errors.
fn set_input_via_cdp(tab: &Arc<Tab>, selector: &str, value: &str) -> Result<()> {
    let focus_script = build_login_input_focus_script(selector)?;
    let focus_result = tab
        .evaluate(&focus_script, true)
        .context("Failed to focus login input")?
        .value
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| anyhow::anyhow!("Login input focus returned no result"))?;
    let focus_status: serde_json::Value = serde_json::from_str(&focus_result)?;
    if focus_status.get("status").and_then(serde_json::Value::as_str) != Some("SUCCESS") {
        anyhow::bail!(
            "Login input is not available for native text entry (reason={})",
            focus_status.get("reason").and_then(serde_json::Value::as_str).unwrap_or("unknown")
        );
    }

    let insert_result = tab.call_method(Input::InsertText {
        text: value.to_string(),
    });

    let verify_script = build_login_input_verify_script(selector, value)?;
    let verify_result = tab
        .evaluate(&verify_script, true)
        .context("Failed to verify login input after native text entry")?
        .value
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| anyhow::anyhow!("Login input verification returned no result"))?;
    let verified: serde_json::Value = serde_json::from_str(&verify_result)?;
    if verified.get("status").and_then(serde_json::Value::as_str) != Some("SUCCESS") {
        let cdp_error = insert_result
            .err()
            .map(|error| format!("; CDP input error: {error:#}"))
            .unwrap_or_default();
        anyhow::bail!(
            "Native text entry did not populate login input (matches={}, length={}){}",
            verified.get("matches").and_then(serde_json::Value::as_bool).unwrap_or(false),
            verified.get("length").and_then(serde_json::Value::as_u64).unwrap_or(0),
            cdp_error
        );
    }
    if let Err(error) = insert_result {
        tracing::warn!(
            "CDP reported an input completion error, but the login field value was verified: {:#}",
            error
        );
    }
    Ok(())
}

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
/// ```no_run
/// use std::sync::Arc;
/// use headless_chrome::Tab;
/// use yasweb_app::yasweb::config::YaswebConfig;
/// use yasweb_app::yasweb::browser::login::execute_login;
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
        Ok(_user_input) => {
            info!("Entering username through native Chrome input events...");
            set_input_via_cdp(tab, username_selector, &config.username)
                .context("Failed to enter and verify username")?;
            info!("Successfully entered and verified username.");

            if let Some(password) = &config.password {
                info!("Waiting for password input...");
                let password_selector = "input[formcontrolname='password'], #passFocus";
                match tab.wait_for_element(password_selector) {
                    Ok(_pass_input) => {
                        info!("Entering password through native Chrome input events...");
                        set_input_via_cdp(tab, password_selector, password)
                            .context("Failed to enter and verify password")?;
                        info!("Successfully entered and verified password.");
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
                    let inner_text = usr_id_element.get_inner_text().unwrap_or_default();
                    if username_matches(&inner_text, &config.username) {
                        login_success = true;
                        info!("Successfully verified configured username on the page.");
                    } else {
                        error!("The username displayed by the portal did not match the configured username.");
                        return Err(anyhow::anyhow!(
                            "Logged-in username did not match configured username"
                        ));
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

#[cfg(test)]
mod native_input_tests {
    use super::{build_login_input_focus_script, build_login_input_verify_script};

    #[test]
    fn focus_script_uses_native_input_and_rejects_unavailable_controls() {
        let script = build_login_input_focus_script("input[formcontrolname='username']").unwrap();
        assert!(script.contains("input.focus()"));
        assert!(script.contains("input.dispatchEvent(new Event(\"input\""));
        assert!(script.contains("input.disabled || input.readOnly"));
        assert!(script.contains("document.activeElement === input"));
    }

    #[test]
    fn verification_script_compares_the_actual_input_value_without_exposing_it() {
        let script = build_login_input_verify_script("#username", "secret-value").unwrap();
        assert!(script.contains("\"secret-value\""));
        assert!(script.contains("input.value ==="));
        assert!(script.contains("matches:"));
        assert!(!script.contains("return JSON.stringify({status: \"SUCCESS\", value: input.value"));
    }

    #[test]
    fn login_selectors_are_json_escaped_before_script_injection() {
        let script = build_login_input_focus_script("input[x=\"; alert(1); //").unwrap();
        assert!(script.contains(r#"\"; alert(1); //"#));
    }
}

#[cfg(test)]
mod tests {
    use super::username_matches;

    #[test]
    fn accepts_exact_username_ignoring_case_and_outer_whitespace() {
        assert!(username_matches("  Mahmoud.Ibrahim ", "mahmoud.ibrahim"));
    }

    #[test]
    fn rejects_username_that_only_contains_configured_username() {
        assert!(!username_matches("other-mahmoud", "mahmoud"));
    }

    #[test]
    fn accepts_numeric_username_with_leading_zeros() {
        assert!(username_matches("00003245", "3245"));
        assert!(username_matches("3245", "00003245"));
        assert!(username_matches("0000", "0"));
    }

    #[test]
    fn rejects_different_numeric_usernames() {
        assert!(!username_matches("00003246", "3245"));
        assert!(!username_matches("", "0"));
    }

    #[test]
    fn rejects_empty_configured_username() {
        assert!(!username_matches("mahmoud", "   "));
    }
}
