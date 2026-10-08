use crate::runner::config::RegisteredApp;
use crate::runner::engine::dispatcher::helpers::{load_config, save_config};

pub async fn create_registered_app(
    path: &str,
    app: RegisteredApp,
) -> anyhow::Result<()> {
    let mut cfg = load_config(path).await?;
    cfg.registered_apps.push(app);
    save_config(cfg, path).await?;
    Ok(())
}

pub async fn update_registered_app(
    path: &str,
    app: RegisteredApp,
) -> anyhow::Result<()> {
    let mut cfg = load_config(path).await?;
    if let Some(existing) = cfg.registered_apps.iter_mut().find(|a| a.id == app.id) {
        existing.name = app.name;
        existing.executable_path = app.executable_path;
        existing.config_path = app.config_path;
        existing.allow_concurrent_tasks = app.allow_concurrent_tasks;
    }
    save_config(cfg, path).await?;
    Ok(())
}

pub async fn delete_registered_app(
    path: &str,
    app_id: &str,
) -> anyhow::Result<()> {
    let mut cfg = load_config(path).await?;
    cfg.registered_apps.retain(|a| a.id != app_id);
    save_config(cfg, path).await?;
    Ok(())
}
