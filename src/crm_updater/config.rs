use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Clone)]
pub struct ReplacementMapEntry {
    pub source_file: String,
    pub target_path: String,
    pub executable_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_args: Option<Vec<String>>,
    #[serde(default = "default_true")]
    pub autostart: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UpdaterConfig {
    pub downloads_dir: String,
    pub runner_logs_dir: String,
    pub log_recipient_email: String,
    pub file_replacement_map: Vec<ReplacementMapEntry>,
    pub log_stdout_level: String,
    pub log_file_level: String,
}

impl fmt::Debug for UpdaterConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UpdaterConfig")
            .field("downloads_dir", &self.downloads_dir)
            .field("runner_logs_dir", &self.runner_logs_dir)
            .field("log_recipient_email", &"***REDACTED***")
            .field("file_replacement_map", &self.file_replacement_map)
            .field("log_stdout_level", &self.log_stdout_level)
            .field("log_file_level", &self.log_file_level)
            .finish()
    }
}

impl fmt::Debug for ReplacementMapEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReplacementMapEntry")
            .field("source_file", &self.source_file)
            .field("target_path", &self.target_path)
            .field("executable_name", &self.executable_name)
            .field("restart_args", &self.restart_args)
            .field("autostart", &self.autostart)
            .finish()
    }
}
