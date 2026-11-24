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
        self.get_semaphore_with_limit(app_id, 1).await
    }

    /// Returns one shared semaphore per registered app. The first caller establishes
    /// the app-wide limit; all task and period executions reuse the same semaphore.
    pub async fn get_semaphore_with_limit(
        &self,
        app_id: &str,
        permits: usize,
    ) -> Arc<Semaphore> {
        let mut mgr = self.inner.lock().await;
        mgr.entry(app_id.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(permits.max(1))))
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::AppLockManager;

    #[tokio::test]
    async fn app_semaphore_limit_is_shared_across_independent_callers() {
        let manager = AppLockManager::new();
        let first = manager.get_semaphore_with_limit("yasweb", 2).await;
        let second = manager.get_semaphore_with_limit("yasweb", 2).await;
        assert!(std::sync::Arc::ptr_eq(&first, &second));

        let permit_a = first.clone().try_acquire_owned().unwrap();
        let permit_b = second.clone().try_acquire_owned().unwrap();
        assert!(first.clone().try_acquire_owned().is_err());
        drop(permit_a);
        assert!(first.clone().try_acquire_owned().is_ok());
        drop(permit_b);
    }
}
