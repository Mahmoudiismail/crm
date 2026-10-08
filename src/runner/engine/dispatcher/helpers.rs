use crate::runner::config::RunnerConfig;
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use tokio::sync::Mutex;

lazy_static! {
    static ref CONFIG_LOCK: Mutex<()> = Mutex::new(());
}

pub(crate) async fn load_config(path: &str) -> Result<RunnerConfig> {
    let _guard = CONFIG_LOCK.lock().await;
    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || RunnerConfig::load(&path_str))
        .await
        .context("spawn_blocking panic for load_config")?
}

pub(crate) async fn save_config(cfg: RunnerConfig, path: &str) -> Result<()> {
    let _guard = CONFIG_LOCK.lock().await;
    let path_str = path.to_string();
    tokio::task::spawn_blocking(move || cfg.save(&path_str))
        .await
        .context("spawn_blocking panic for save_config")?
}

pub(crate) async fn modify_config<F>(path: &str, f: F) -> Result<()>
where
    F: FnOnce(&mut RunnerConfig) + Send + 'static,
{
    let _guard = CONFIG_LOCK.lock().await;
    let path_str = path.to_string();
    let path_str_save = path.to_string();
    tokio::task::spawn_blocking(move || {
        let mut cfg = RunnerConfig::load(&path_str)?;
        f(&mut cfg);
        cfg.save(&path_str_save)
    })
    .await
    .context("spawn_blocking panic for modify_config")??;
    Ok(())
}
