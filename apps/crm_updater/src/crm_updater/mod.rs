pub mod config;
pub mod logs;
pub mod update;

/// Install only CRM Updater's built-in PowerShell templates.
pub fn install_builtin_scripts() -> anyhow::Result<()> {
    let manager = crm_tool::script_manager::ScriptManager::new();
    let scripts: &[(&str, &str, &str)] = &[
        ("CRM Updater", "scan_update_drafts.ps1", include_str!("scripts/scan_update_drafts.ps1")),
        ("CRM Updater", "apply_update.ps1", include_str!("scripts/apply_update.ps1")),
        ("CRM Updater", "unblock_file.ps1", include_str!("scripts/unblock_file.ps1")),
    ];
    for (task_name, logical_name, content) in scripts {
        manager.get_or_create_script(task_name, logical_name, content)?;
    }
    Ok(())
}
