use crate::runner::config::models::RunnerConfig;
use anyhow::{Context, Result};

impl RunnerConfig {
    pub fn load(path: &str) -> Result<Self> {
        let config_path = std::path::Path::new(path);
        let config: Self = crate::utils::load_or_create_config(config_path, &Self::default())?;
        Ok(config)
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let pretty = serde_json::to_string_pretty(self)?;

        if let Ok(existing_content) = std::fs::read_to_string(path) {
            if existing_content == pretty {
                tracing::debug!("Config unchanged, skipping file write");
                return Ok(());
            }
            // Fallback to value equality in case of formatting differences
            if let (Ok(existing_val), Ok(new_val)) = (
                serde_json::from_str::<serde_json::Value>(&existing_content),
                serde_json::from_str::<serde_json::Value>(&pretty),
            ) {
                if existing_val == new_val {
                    tracing::debug!("Config unchanged, skipping file write");
                    return Ok(());
                }
            }
        }

        crate::utils::atomic_write(std::path::Path::new(path), &pretty)
            .with_context(|| format!("Failed to write runner config: {}", path))?;
        Ok(())
    }
}
