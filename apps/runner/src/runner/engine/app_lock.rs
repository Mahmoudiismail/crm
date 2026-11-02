use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

#[derive(Clone, Default)]
pub struct AppLockManager {
    inner: Arc<Mutex<HashMap<String, Arc<Semaphore>>>>,
}

impl std::fmt::Debug for AppLockManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppLockManager").finish()
    }
}

impl AppLockManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_semaphore(&self, app_id: &str) -> Arc<Semaphore> {
        let mut mgr = self.inner.lock().await;
        mgr.entry(app_id.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(1)))
            .clone()
    }
}
