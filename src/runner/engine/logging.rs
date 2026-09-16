use std::fs;
use std::io::Write;
use std::sync::Arc;

use chrono::Local;
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
        inner.log(message);
    }

    pub async fn log_bytes(&self, prefix: &str, bytes: &[u8]) {
        let mut inner = self.inner.lock().await;
        inner.log_bytes(prefix, bytes);
    }

    pub async fn log_path_async(&self) -> std::path::PathBuf {
        let inner = self.inner.lock().await;
        inner.log_path.clone().unwrap_or_default()
    }
}

#[derive(Debug)]
struct TaskLoggerInner {
    file: Option<fs::File>,
    task_id: String,
    log_path: Option<std::path::PathBuf>,
}

impl TaskLoggerInner {
    fn new(task_id: &str, task_name: &str) -> Self {
        let now = Local::now();
        let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
        let safe_task_name = task_name.replace(|c: char| !c.is_alphanumeric(), "_");
        let filename = format!("{}_{}_{}.log", timestamp, safe_task_name, task_id);

        let log_dir = match std::env::current_exe() {
            Ok(exe) => exe
                .parent()
                .map(|p| p.join("logs").join(&safe_task_name))
                .unwrap_or_else(|| std::path::PathBuf::from("logs").join(&safe_task_name)),
            Err(_) => std::path::PathBuf::from("logs").join(&safe_task_name),
        };

        if let Err(e) = std::fs::create_dir_all(&log_dir) {
            error!("Failed to create log directory {}: {}", log_dir.display(), e);
            return Self {
                file: None,
                task_id: task_id.to_string(),
                log_path: None,
            };
        }

        let log_path = log_dir.join(filename);
        match std::fs::File::create(&log_path) {
            Ok(file) => {
                let mut logger = Self {
                    file: Some(file),
                    task_id: task_id.to_string(),
                    log_path: Some(log_path.clone()),
                };
                logger.log("==================================================");
                logger.log(&format!("TASK INITIATED: {} (ID: {})", task_name, task_id));
                logger.log(&format!("START TIME:     {}", now.to_rfc3339()));
                logger.log("==================================================");
                logger
            }
            Err(e) => {
                error!("Failed to create log file {}: {}", log_path.display(), e);
                Self {
                    file: None,
                    task_id: task_id.to_string(),
                    log_path: None,
                }
            }
        }
    }

    fn log(&mut self, message: &str) {
        let now = Local::now().to_rfc3339();
        let line = format!("[{}] {}\n", now, message);
        if let Some(ref mut f) = self.file {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
        debug!("[Task:{}] {}", self.task_id, message);
    }

    fn log_file_only(&mut self, message: &str) {
        let now = Local::now().to_rfc3339();
        let line = format!("[{}] {}\n", now, message);
        if let Some(ref mut f) = self.file {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }

    fn log_bytes(&mut self, prefix: &str, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(bytes);
        for line in text.lines() {
            self.log_file_only(&format!("{}: {}", prefix, line));
        }
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
    async fn test_task_logger_isolation_and_path() {
        let logger = TaskLogger::new("task1", "Test Task");
        logger.log("Hello from Task 1").await;
        logger.log_bytes("STDOUT", b"Line 1\nLine 2").await;

        let path = logger.log_path_async().await;
        assert!(path.exists(), "Log file should be created correctly");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("TASK INITIATED: Test Task"));
        assert!(content.contains("Hello from Task 1"));
        assert!(content.contains("STDOUT: Line 1"));
        assert!(content.contains("STDOUT: Line 2"));
    }
}
