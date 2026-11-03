pub fn is_valid_task_id(value: &str) -> bool {
    value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

pub fn excerpt_utf8(bytes: &[u8]) -> String {
    const MAX: usize = 400;
    let text = String::from_utf8_lossy(bytes).replace(['\n', '\r'], " ");
    if text.len() > MAX {
        format!("{}...", &text[..MAX])
    } else if text.is_empty() {
        "<empty>".to_string()
    } else {
        text
    }
}
