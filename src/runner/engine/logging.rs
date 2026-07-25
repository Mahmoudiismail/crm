use std::fs;
use std::io::Write;
use std::sync::Arc;

use chrono::Utc;
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
}

#[derive(Debug)]
struct TaskLoggerInner {
    file: Option<fs::File>,
    task_id: String,
}

impl TaskLoggerInner {
    fn new(task_id: &str, task_name: &str) -> Self {
        let now = Utc::now();
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

        if let Err(e) = fs::create_dir_all(&log_dir) {
            error!(
                "Failed to create log directory {}: {}",
                log_dir.display(),
                e
            );
            return Self {
                file: None,
                task_id: task_id.to_string(),
            };
        }

        let log_path = log_dir.join(filename);
        match fs::File::create(&log_path) {
            Ok(file) => {
                let mut logger = Self {
                    file: Some(file),
                    task_id: task_id.to_string(),
                };
                logger.log(&format!("Task ID: {}", task_id));
                logger.log(&format!("Task Name: {}", task_name));
                logger.log(&format!("Start Time: {}", now.to_rfc3339()));
                logger.log("--------------------------------------------------");
                logger
            }
            Err(e) => {
                error!("Failed to create log file {}: {}", log_path.display(), e);
                Self {
                    file: None,
                    task_id: task_id.to_string(),
                }
            }
        }
    }

    fn log(&mut self, message: &str) {
        let now = Utc::now().to_rfc3339();
        let line = format!("[{}] {}\n", now, message);
        if let Some(ref mut f) = self.file {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
        // Capture into the runner's own log for unified tracing
        debug!("[Task:{}] {}", self.task_id, message);
    }

    fn log_bytes(&mut self, prefix: &str, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(bytes);
        for line in text.lines() {
            self.log(&format!("{}: {}", prefix, line));
        }
    }
}
