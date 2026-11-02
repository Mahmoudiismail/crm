use anyhow::Result;
pub use crm_tool::script_manager::{ScriptEntry, ScriptManager, TaskMetadata};

/// Install only Tasker's built-in PowerShell templates.
pub fn install_builtin_scripts() -> Result<()> {
    let manager = ScriptManager::new();
    let scripts: &[(&str, &str, &str)] = &[
        (
            "Email",
            "outlook_mail.ps1",
            include_str!("email/outlook_mail.ps1"),
        ),
        (
            "Dashboard Updater",
            "dashboard_updater.ps1",
            include_str!("scripts/dashboard_updater.ps1"),
        ),
        (
            "Department Split",
            "department_split.ps1",
            include_str!("scripts/department_split.ps1"),
        ),
        (
            "CRM Open Sohail",
            "slicer_extract.ps1",
            include_str!("crm_open_sohail/slicer_extract.ps1"),
        ),
        (
            "OPD Analysis",
            "opd_analysis_email.ps1",
            include_str!("opd_task/opd_analysis_email.ps1"),
        ),
        (
            "Sat Figures",
            "sat_figures_email.ps1",
            include_str!("sat_figures_email.ps1"),
        ),
    ];
    for (task_name, logical_name, content) in scripts {
        manager.get_or_create_script(task_name, logical_name, content)?;
    }
    Ok(())
}
