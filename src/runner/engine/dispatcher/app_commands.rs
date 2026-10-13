use crate::runner::config::RegisteredApp;

pub async fn create_registered_app(path: &str, app: RegisteredApp) -> anyhow::Result<()> {
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        cfg.registered_apps.push(app);
        Ok(())
    }).await
}

pub async fn update_registered_app(path: &str, app: RegisteredApp) -> anyhow::Result<()> {
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        if let Some(existing) = cfg.registered_apps.iter_mut().find(|a| a.id == app.id) {
            existing.name = app.name;
            existing.executable_path = app.executable_path;
            existing.config_path = app.config_path;
            existing.allow_concurrent_tasks = app.allow_concurrent_tasks;
            Ok(())
        } else {
            Err(anyhow::anyhow!("App '{}' not found", app.id))
        }
    }).await
}

pub async fn delete_registered_app(path: &str, app_id: &str) -> anyhow::Result<()> {
    let aid = app_id.to_string();
    crate::runner::engine::dispatcher::helpers::modify_config(path, move |cfg| {
        cfg.registered_apps.retain(|a| a.id != aid);
        Ok(())
    }).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::config::RunnerConfig;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_update_nonexistent_registered_app_fails() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("runner.json");
        let path_str = config_path.to_str().unwrap();

        let cfg = RunnerConfig::default();
        cfg.save(path_str).unwrap();

        let app = RegisteredApp {
            id: "fake_id".to_string(),
            name: "fake".to_string(),
            executable_path: "fake.exe".to_string(),
            config_path: "fake.json".to_string(),
            allow_concurrent_tasks: false,
        };

        let res = update_registered_app(path_str, app).await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "App 'fake_id' not found");
    }
}
