use anyhow::Result;
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
            if let Some(v) = res.value {
                if let Some(s) = v.as_str() {
                    let parsed: serde_json::Value = serde_json::from_str(s).unwrap_or_default();
                    let status = parsed
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("UNKNOWN");
                    let logs = parsed
                        .get("logs")
                        .map(|v| v.to_string())
                        .unwrap_or_default();

                    if status == "ERROR" {
                        let msg = parsed
                            .get("msg")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown error");
                        error!("{} Failed: {} | Logs: {}", step_name, msg, logs);
                        return Err(anyhow::anyhow!("Automation {} failed", step_name));
                    } else {
                        info!("{} Success | Logs: {}", step_name, logs);
                        return Ok(parsed);
                    }
                }
            }
            error!("Failed to extract result string from {} JS.", step_name);
            Err(anyhow::anyhow!("Automation {} failed", step_name))
        }
        Err(e) => {
            error!("Failed to evaluate {} JS: {:?}", step_name, e);
            Err(anyhow::anyhow!("Automation {} failed", step_name))
        }
    }
}
