use crate::runner::config::RunnerConfig;
use anyhow::{Context, Result};

pub(crate) async fn load_config(path: &str) -> Result<RunnerConfig> {
    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic for load_config")?
}

pub(crate) async fn save_config(cfg: RunnerConfig, path: &str) -> Result<()> {
    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || cfg.save(&path_str))
        .await
        .context("spawn_blocking panic for save_config")?
}
