use anyhow::{Context, Result};
use headless_chrome::Tab;
use std::sync::Arc;
use tracing::{error, info};

pub fn evaluate_automation_step(
    tab: &Arc<Tab>,
    js_code: &str,
    step_name: &str,
) -> Result<serde_json::Value> {
    match tab.evaluate(js_code, true) {
        Ok(res) => {
            let value = res
                .value
                .and_then(|value| value.as_str().map(str::to_owned))
                .with_context(|| format!("Automation {} returned no result string", step_name))?;

            parse_automation_result(&value, step_name)
        }
        Err(e) => {
            error!("Failed to evaluate {} JS: {:?}", step_name, e);
            Err(anyhow::anyhow!("Automation {} failed", step_name))
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
            Err(anyhow::anyhow!("Automation {} failed", step_name))
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
    use super::parse_automation_result;

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
    fn error_status_is_an_error() {
        assert!(parse_automation_result(
            r#"{"status":"ERROR","msg":"failed","logs":[]}"#,
            "test step"
        )
        .is_err());
    }
}
