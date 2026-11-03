pub fn with_retry<T, E, F>(mut f: F) -> std::result::Result<T, E>
where
    F: FnMut() -> std::result::Result<T, E>,
    E: std::fmt::Display,
{
    match f() {
        Ok(t) => Ok(t),
        Err(e) => {
            tracing::warn!("Task phase failed on attempt 1: {}. Retrying...", e);
            f()
        }
    }
}
