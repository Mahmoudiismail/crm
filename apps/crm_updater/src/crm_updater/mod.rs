pub mod config;
pub mod logs;
pub mod update;

fn install_with_manager(manager: &crm_tool::script_manager::ScriptManager) -> anyhow::Result<()> {
    let scripts: &[(&str, &str, &str)] = &[
        (
            "CRM Updater",
            "scan_update_drafts.ps1",
            include_str!("scripts/scan_update_drafts.ps1"),
        ),
        (
            "CRM Updater",
            "apply_update.ps1",
            include_str!("scripts/apply_update.ps1"),
        ),
        (
            "CRM Updater",
            "unblock_file.ps1",
            include_str!("scripts/unblock_file.ps1"),
        ),
    ];
    for (task_name, logical_name, content) in scripts {
        manager.get_or_create_script(task_name, logical_name, content)?;
    }
    Ok(())
}

/// Install only CRM Updater's built-in PowerShell templates.
pub fn install_builtin_scripts() -> anyhow::Result<()> {
    install_with_manager(&crm_tool::script_manager::ScriptManager::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn installs_only_updater_owned_templates() {
        let root = tempdir().unwrap();
        let manager = crm_tool::script_manager::ScriptManager::with_root_dir(root.path());

        install_with_manager(&manager).unwrap();

        assert!(root
            .path()
            .join("CRM Updater")
            .join("apply_update.ps1")
            .is_file());
        assert!(!root.path().join("Email").exists());
    }
}
