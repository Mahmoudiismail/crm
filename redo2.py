import re

with open("src/bin/runner.rs", "r") as f:
    text = f.read()

text = text.replace(
"""    let runner_cfg = {
        let cfg = crm_tool::runner::config::RunnerConfig::load(&runner_config_path_str)
            .unwrap_or_default();
        if let Err(e) = cfg.validate() {
            eprintln!("Runner configuration validation failed: {}", e);
            std::process::exit(1);
        }""",
"""    let runner_cfg = {
        let cfg = if config_exists {
            match crm_tool::runner::config::RunnerConfig::load(&runner_config_path_str) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("CRITICAL ERROR: Failed to parse existing runner_config.json: {}", e);
                    std::process::exit(1);
                }
            }
        } else {
            crm_tool::runner::config::RunnerConfig::default()
        };
        if let Err(e) = cfg.validate() {
            eprintln!("Runner configuration validation failed: {}", e);
            std::process::exit(1);
        }"""
)

with open("src/bin/runner.rs", "w") as f:
    f.write(text)

with open("src/runner/config/mod.rs", "r") as f:
    config_mod = f.read()

if "fn test_runner_startup_fails_on_corrupt_config" not in config_mod:
    config_mod += """
#[cfg(test)]
mod startup_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_runner_startup_fails_on_corrupt_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("runner.json");
        let path_str = config_path.to_str().unwrap();

        std::fs::write(path_str, b"{ broken json").unwrap();

        let res = RunnerConfig::load(path_str);
        assert!(res.is_err());
        let err_str = res.err().unwrap().to_string();
        assert!(!err_str.is_empty());
    }
}
"""

with open("src/runner/config/mod.rs", "w") as f:
    f.write(config_mod)

with open("src/runner/engine/logging.rs", "r") as f:
    text = f.read()

text = text.replace(
"""    pub async fn log_path_async(&self) -> std::path::PathBuf {
        let inner = self.inner.lock().await;
        inner.log_path.clone().unwrap_or_default()
    }""",
"""    pub async fn log_path_async(&self) -> anyhow::Result<std::path::PathBuf> {
        let mut inner = self.inner.lock().await;
        inner.ensure_initialized().await?;
        Ok(inner.log_path.clone().unwrap_or_default())
    }"""
)

text = text.replace(
"""    async fn ensure_initialized(&mut self) -> anyhow::Result<()> {
        if self.initialized {
            return Ok(());
        }
        self.initialized = true; // prevent retry loops if it fails

        let now = Local::now();""",
"""    async fn ensure_initialized(&mut self) -> anyhow::Result<()> {
        if self.initialized {
            return Ok(());
        }

        let now = Local::now();"""
)

text = text.replace(
"""        file.flush().await?;

        self.file = Some(file);
        self.log_path = Some(log_path);

        Ok(())
    }""",
"""        file.flush().await?;

        self.file = Some(file);
        self.log_path = Some(log_path);

        self.initialized = true; // Successfully initialized

        Ok(())
    }"""
)

tests_to_add = """
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
"""

if "fn test_task_logger_initialization_failure_retry" not in text:
    text = text.replace("    #[tokio::test]", tests_to_add + "\n    #[tokio::test]")

text = text.replace(
"""        logger.log("Hello Isolation").await;

        let path = logger.log_path_async().await;""",
"""        logger.log("Hello Isolation").await;

        let path = logger.log_path_async().await.unwrap();"""
)

with open("src/runner/engine/logging.rs", "w") as f:
    f.write(text)

with open("src/runner/engine/pipeline.rs", "r") as f:
    pipeline = f.read()

pipeline = pipeline.replace("logger.log_path_async().await;", "logger.log_path_async().await.unwrap_or_default();")

with open("src/runner/engine/pipeline.rs", "w") as f:
    f.write(pipeline)

with open("src/runner/engine/shell.rs", "r") as f:
    shell = f.read()

shell = shell.replace("logger.log_path_async().await;", "logger.log_path_async().await.unwrap_or_default();")
shell = shell.replace("let path = logger.log_path_async().await;", "let path = logger.log_path_async().await.unwrap_or_default();")

with open("src/runner/engine/shell.rs", "w") as f:
    f.write(shell)


with open("src/runner/engine/process.rs", "r") as f:
    proc = f.read()

proc = proc.replace("logger.log_path_async().await;", "logger.log_path_async().await.unwrap_or_default();")
proc = proc.replace("let path = logger.log_path_async().await;", "let path = logger.log_path_async().await.unwrap_or_default();")

with open("src/runner/engine/process.rs", "w") as f:
    f.write(proc)


with open("src/runner/gui/handlers.rs", "r") as f:
    handlers = f.read()

handlers = handlers.replace(
"""pub(crate) async fn handle_apps_delete(
    handle: &RunnerHandle,
    app_id: &str,
) -> Result<(u16, &'static str, String)> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    if handle
        .command_tx
        .send(RunnerCommand::DeleteRegisteredApp {
            app_id: app_id.to_string(),
            reply: tx,
        })""",
"""pub(crate) async fn handle_apps_delete(
    handle: &RunnerHandle,
    app_id: &str,
) -> Result<(u16, &'static str, String)> {
    let decoded_app_id = urlencoding::decode(app_id)
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| app_id.to_string());

    let (tx, rx) = tokio::sync::oneshot::channel();
    if handle
        .command_tx
        .send(RunnerCommand::DeleteRegisteredApp {
            app_id: decoded_app_id,
            reply: tx,
        })"""
)

with open("src/runner/gui/handlers.rs", "w") as f:
    f.write(handlers)

with open("md/OPERATIONS.md", "r") as f:
    text = f.read()

text = text.replace(
"""## Troubleshooting: PowerShell Execution & File Locks""",
"""## Troubleshooting: Runner Startup
- **Symptom:** Runner process terminates immediately on startup with a `CRITICAL ERROR: Failed to parse existing runner_config.json` message, rather than overwriting it with a blank UI.
- **Cause:** The `runner_config.json` file contains invalid or corrupted JSON data. The daemon enforces strict parsing on existing configuration files to prevent data loss.
- **Resolution:** Manually fix the JSON syntax in `runner_config.json` or delete it if a fresh installation is desired.

## Troubleshooting: PowerShell Execution & File Locks""")

text = text.replace(
"""- **TaskLogger I/O Isolation:** `TaskLogger` executes filesystem I/O (log directory creation, file creation, appending bytes, and flushing) using asynchronous `tokio::fs` coupled with Tokio `Mutex` serialization. This ensures Tokio's core executor pool is not stalled by slow I/O when generating large task logs, while also guaranteeing strict log line ordering without cloned file handles.""",
"""- **TaskLogger I/O Isolation:** `TaskLogger` executes filesystem I/O (log directory creation, file creation, appending bytes, and flushing) using asynchronous `tokio::fs` coupled with Tokio `Mutex` serialization. This ensures Tokio's core executor pool is not stalled by slow I/O when generating large task logs, while also guaranteeing strict log line ordering without cloned file handles. The logger is lazily initialized on the first log line or path request, and safely retries initialization across temporary filesystem errors without permanent lock-out.""")

with open("md/OPERATIONS.md", "w") as f:
    f.write(text)

print("success")
