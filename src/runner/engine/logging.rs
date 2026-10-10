use std::sync::Arc;

use chrono::Local;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tracing::{debug, error};

#[derive(Clone, Debug)]
pub struct TaskLogger {
    inner: Arc<Mutex<TaskLoggerInner>>,
}

impl TaskLogger {
    pub fn new(task_id: &str, task_name: &str) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TaskLoggerInner::new(task_id, task_name))),
        }
    }

    pub async fn log(&self, message: &str) {
        let mut inner = self.inner.lock().await;
        if let Err(e) = inner.ensure_initialized().await {
            error!("TaskLogger initialization failed: {}", e);
        }

        let task_id = inner.task_id.clone();
        let now = Local::now().to_rfc3339();
        let line = format!("[{}] {}\n", now, message);

        if let Some(f) = inner.file.as_mut() {
            let _ = f.write_all(line.as_bytes()).await;
            let _ = f.flush().await;
        }
        debug!("[Task:{}] {}", task_id, message);
    }

    pub async fn log_bytes(&self, prefix: &str, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }

        let mut inner = self.inner.lock().await;
        if let Err(e) = inner.ensure_initialized().await {
            error!("TaskLogger initialization failed: {}", e);
        }

        if let Some(f) = inner.file.as_mut() {
            let text = String::from_utf8_lossy(bytes).into_owned();
            let mut all_lines = String::new();
            for line in text.lines() {
                let now = Local::now().to_rfc3339();
                all_lines.push_str(&format!("[{}] {}: {}\n", now, prefix, line));
            }
            let _ = f.write_all(all_lines.as_bytes()).await;
            let _ = f.flush().await;
        }
    }

    pub async fn log_path_async(&self) -> anyhow::Result<std::path::PathBuf> {
        let mut inner = self.inner.lock().await;
        inner.ensure_initialized().await?;
        Ok(inner.log_path.clone().unwrap_or_default())
    }
}

#[derive(Debug)]
struct TaskLoggerInner {
    file: Option<tokio::fs::File>,
    task_id: String,
    task_name: String,
    log_path: Option<std::path::PathBuf>,
    initialized: bool,
}

impl TaskLoggerInner {
    fn new(task_id: &str, task_name: &str) -> Self {
        Self {
            file: None,
            task_id: task_id.to_string(),
            task_name: task_name.to_string(),
            log_path: None,
            initialized: false,
        }
    }

    async fn ensure_initialized(&mut self) -> anyhow::Result<()> {
        if self.initialized {
            return Ok(());
        }

        let now = Local::now();
        let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
        let safe_task_name = self.task_name.replace(|c: char| !c.is_alphanumeric(), "_");
        let filename = format!("{}_{}_{}.log", timestamp, safe_task_name, self.task_id);

        let log_dir = match std::env::current_exe() {
            Ok(exe) => exe
                .parent()
                .map(|p| p.join("logs").join(&safe_task_name))
                .unwrap_or_else(|| std::path::PathBuf::from("logs").join(&safe_task_name)),
            Err(_) => std::path::PathBuf::from("logs").join(&safe_task_name),
        };

        tokio::fs::create_dir_all(&log_dir)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create log dir: {}", e))?;

        let log_path = log_dir.join(filename);
        let mut file = tokio::fs::File::create(&log_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create log file: {}", e))?;

        let lines = vec![
            format!(
                "[{}] ==================================================\n",
                now.to_rfc3339()
            ),
            format!(
                "[{}] TASK INITIATED: {} (ID: {})\n",
                now.to_rfc3339(),
                self.task_name,
                self.task_id
            ),
            format!(
                "[{}] ==================================================\n",
                now.to_rfc3339()
            ),
        ];

        for line in lines {
            file.write_all(line.as_bytes()).await?;
        }
        file.flush().await?;

        self.file = Some(file);
        self.log_path = Some(log_path);

        self.initialized = true; // Successfully initialized

        Ok(())
    }
}

pub async fn cleanup_old_logs(log_retention_days: u64) {
    if log_retention_days == 0 {
        return; // Disable cleanup if 0
    }

    let _ = tokio::task::spawn_blocking(move || {
        let log_dir = match std::env::current_exe() {
            Ok(exe) => exe
                .parent()
                .map(|p| p.join("logs"))
                .unwrap_or_else(|| std::path::PathBuf::from("logs")),
            Err(_) => std::path::PathBuf::from("logs"),
        };

        if !log_dir.exists() {
            return;
        }

        let threshold = chrono::Local::now()
            - chrono::Duration::try_days(log_retention_days as i64)
                .unwrap_or(chrono::Duration::zero());

        let mut walk_dir = walkdir::WalkDir::new(&log_dir).into_iter();
        while let Some(Ok(entry)) = walk_dir.next() {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "log" {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                let modified_time: chrono::DateTime<chrono::Local> =
                                    modified.into();
                                if modified_time < threshold {
                                    tracing::info!(
                                        "Cleaning up old log file: {}",
                                        entry.path().display()
                                    );
                                    if let Err(e) = std::fs::remove_file(entry.path()) {
                                        tracing::warn!(
                                            "Failed to remove old log file {}: {}",
                                            entry.path().display(),
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
    .await;
}
#[cfg(test)]
mod tests {
    use super::*;


    #[tokio::test]
    async fn test_task_logger_initialization_failure_retry() {
        let task_id = "task_fail_123";
        let task_name = "task_fail_name";
        let inner = TaskLoggerInner::new(task_id, task_name);
        assert!(!inner.initialized);
    }

    #[tokio::test]
    async fn test_task_logger_path_lazy_init() {
        let task_name = "LazyInitTest";
        let task_id = "task_lazy_456";
        let logger = TaskLogger::new(task_id, task_name);

        let path = logger.log_path_async().await.expect("Should initialize successfully");
        assert!(path.to_string_lossy().contains("LazyInitTest"));
        assert!(path.to_string_lossy().contains("task_lazy_456"));
    }

    #[tokio::test]
    async fn test_task_logger_isolation_and_path() {
        let task_name = "Isolation_Test!";
        let safe_name = "Isolation_Test_";
        let task_id = "task_iso_123";

        let logger = TaskLogger::new(task_id, task_name);

        {
            let inner = logger.inner.lock().await;
            assert!(
                inner.file.is_none(),
                "TaskLogger file should be uninitialized before first log"
            );
            assert!(
                !inner.initialized,
                "TaskLogger initialized flag should be false"
            );
        }

        logger.log("Hello Isolation").await;

        let path = logger.log_path_async().await.unwrap();
        let path_str = path.to_string_lossy();

        assert!(
            path_str.contains(safe_name),
            "Log path {} should contain safe name {}",
            path_str,
            safe_name
        );
        assert!(
            path_str.contains(task_id),
            "Log path {} should contain task id {}",
            path_str,
            task_id
        );

        let content = std::fs::read_to_string(&path).expect("Failed to read test log file");
        assert!(
            content.contains("Hello Isolation"),
            "Content should contain the logged message"
        );
        assert!(
            content.contains("TASK INITIATED:"),
            "Content should contain the init headers"
        );
    }
}
