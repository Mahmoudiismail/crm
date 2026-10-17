use anyhow::{Context, Result};
use headless_chrome::Tab;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info};

fn prepare_automation_script(js_code: &str) -> String {
    // Runtime.evaluate expects an expression, not a top-level return statement.
    // Wrap return-await snippets in an async function so CDP can await the Promise
    // and expose the final JSON string to the Rust caller.
    if js_code.trim_start().starts_with("return await ") {
        format!("(async () => {{ {} }})()", js_code)
    } else {
        js_code.to_owned()
    }
}

pub fn evaluate_automation_step(
    tab: &Arc<Tab>,
    js_code: &str,
    step_name: &str,
) -> Result<serde_json::Value> {
    let started = Instant::now();
    let evaluation_script = prepare_automation_script(js_code);
    info!(
        "AUTOMATION STEP START: {} (javascript_bytes={})",
        step_name,
        js_code.len()
    );
    match tab.evaluate(&evaluation_script, true) {
        Ok(res) => {
            let value = match res
                .value
                .and_then(|value| value.as_str().map(str::to_owned))
            {
                Some(value) => value,
                None => {
                    let message = format!("Automation {} returned no result string", step_name);
                    error!(
                        "AUTOMATION STEP END: {} status=NO_RESULT elapsed_ms={} error={}",
                        step_name,
                        started.elapsed().as_millis(),
                        message
                    );
                    crate::yasweb::browser::debug::save_failure_diagnostics(tab, step_name);
                    return Err(anyhow::anyhow!(message));
                }
            };

            match parse_automation_result(&value, step_name) {
                Ok(result) => {
                    info!(
                        "AUTOMATION STEP END: {} status=SUCCESS elapsed_ms={}",
                        step_name,
                        started.elapsed().as_millis()
                    );
                    Ok(result)
                }
                Err(e) => {
                    error!(
                        "AUTOMATION STEP END: {} status=ERROR elapsed_ms={} error={:#}",
                        step_name,
                        started.elapsed().as_millis(),
                        e
                    );
                    crate::yasweb::browser::debug::save_failure_diagnostics(tab, step_name);
                    Err(e)
                }
            }
        }
        Err(e) => {
            error!(
                "AUTOMATION STEP END: {} status=EVALUATION_ERROR elapsed_ms={} error={:?}",
                step_name,
                started.elapsed().as_millis(),
                e
            );
            crate::yasweb::browser::debug::save_failure_diagnostics(tab, step_name);
            Err(anyhow::anyhow!(
                "Automation {} JavaScript evaluation failed: {}",
                step_name,
                e
            ))
        }
    }
}

fn parse_automation_result(raw: &str, step_name: &str) -> Result<serde_json::Value> {
    let parsed: serde_json::Value = serde_json::from_str(raw)
        .with_context(|| format!("Automation {} returned invalid JSON", step_name))?;
    let status = parsed
        .get("status")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Automation {} returned no valid status", step_name))?;

    let logs = parsed
        .get("logs")
        .map(serde_json::Value::to_string)
        .unwrap_or_default();

    match status {
        "ERROR" => {
            let msg = parsed
                .get("msg")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Unknown error");
            error!("{} Failed: {} | Logs: {}", step_name, msg, logs);
            Err(anyhow::anyhow!(
                "Automation {} failed: {} | JavaScript logs: {}",
                step_name,
                msg,
                logs
            ))
        }
        "SUCCESS" => {
            info!("{} Success | Logs: {}", step_name, logs);
            Ok(parsed)
        }
        _ => Err(anyhow::anyhow!(
            "Automation {} returned unsupported status",
            step_name
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_automation_result, prepare_automation_script};

    #[test]
    fn return_await_scripts_are_wrapped_as_awaitable_expressions() {
        let script = prepare_automation_script(
            r#"return await (async function() { return JSON.stringify({ status: "SUCCESS", logs: [] }); })()"#,
        );
        assert!(script.starts_with("(async () => { return await "));
        assert!(script.ends_with("})()"));
    }

    #[test]
    fn ordinary_expression_scripts_are_preserved() {
        let source = r#"(async function() { return JSON.stringify({ status: "SUCCESS", logs: [] }); })()"#;
        assert_eq!(prepare_automation_script(source), source);
    }

    #[test]
    fn malformed_json_is_an_error() {
        assert!(parse_automation_result("{not json", "test step").is_err());
    }

    #[test]
    fn missing_status_is_an_error() {
        assert!(parse_automation_result(r#"{"msg":"done"}"#, "test step").is_err());
    }

    #[test]
    fn unknown_status_is_an_error() {
        assert!(parse_automation_result(r#"{"status":"UNKNOWN"}"#, "test step").is_err());
    }

    #[test]
    fn success_status_returns_parsed_result() {
        let result = parse_automation_result(
            r#"{"status":"SUCCESS","msg":"done","logs":[]}"#,
            "test step",
        )
        .expect("SUCCESS should be accepted");

        assert_eq!(result["status"], "SUCCESS");
    }

    #[test]
    fn error_status_preserves_message_and_browser_logs() {
        let error = parse_automation_result(
            r#"{"status":"ERROR","msg":"Report list timeout","logs":["waiting for tree-view","search input missing"]}"#,
            "Step 2 (Select Report)",
        )
        .expect_err("ERROR status must fail")
        .to_string();

        assert!(error.contains("Report list timeout"));
        assert!(error.contains("waiting for tree-view"));
        assert!(error.contains("search input missing"));
    }

    #[test]
    fn error_status_is_an_error() {
        assert!(parse_automation_result(
            r#"{"status":"ERROR","msg":"failed","logs":[]}"#,
            "test step"
        )
        .is_err());
    }
}
