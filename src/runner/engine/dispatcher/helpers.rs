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



pub(crate) async fn modify_config<F, T>(path: &str, f: F) -> Result<T>
where
    F: FnOnce(&mut RunnerConfig) -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    let _guard = CONFIG_LOCK.lock().await;
    let path_str = path.to_string();
    let path_str_save = path.to_string();
    let res: Result<T> = tokio::task::spawn_blocking(move || {
        let mut cfg = RunnerConfig::load(&path_str)?;
        let r = f(&mut cfg)?;
        cfg.save(&path_str_save)?;
        Ok(r)
    })
    .await
    .context("spawn_blocking panic for modify_config")?;
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_modify_config_concurrency_race() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("config.json");
        let path_str = path.to_str().unwrap();

        let cfg = RunnerConfig {
            poll_interval_seconds: 0,
            ..RunnerConfig::default()
        };
        cfg.save(path_str).unwrap();

        let mut handles = vec![];
        for _ in 0..10 {
            let p = path_str.to_string();
            handles.push(tokio::spawn(async move {
                modify_config(&p, |c| {
                    c.poll_interval_seconds += 1;
                    Ok(())
                }).await.unwrap();
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let final_cfg = load_config(path_str).await.unwrap();
        assert_eq!(final_cfg.poll_interval_seconds, 10);
    }
}
