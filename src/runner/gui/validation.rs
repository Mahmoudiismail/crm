use std::collections::HashMap;

pub(crate) fn parse_checkbox(values: &HashMap<String, String>, key: &str) -> bool {
    values
        .get(key)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}
