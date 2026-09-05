import re

# 1. CsvAnalysis
with open("src/tasker/csv_task/mod.rs", "r") as f:
    code = f.read()

code = code.replace("use anyhow::{Context, Result};", "use anyhow::{Context, Result};\nuse crate::tasker::utils::with_retry;")

old_dl = "let csv_files = download_reports(config)?;"
new_dl = "let csv_files = with_retry(|| download_reports(config))?;"
code = code.replace(old_dl, new_dl)

old_write = """    let mut out_file = std::fs::File::create(&config.output_file)
        .with_context(|| format!("Failed to create output file: {}", config.output_file))?;
    for row in &output_rows {
        writeln!(out_file, "{}", row.join(","))
            .with_context(|| format!("Failed to write row to output file: {:?}", row))?;
    }"""
new_write = """    with_retry(|| -> Result<()> {
        let mut out_file = std::fs::File::create(&config.output_file)
            .with_context(|| format!("Failed to create output file: {}", config.output_file))?;
        for row in &output_rows {
            writeln!(out_file, "{}", row.join(","))
                .with_context(|| format!("Failed to write row to output file: {:?}", row))?;
        }
        Ok(())
    })?;"""
code = code.replace(old_write, new_write)

old_email = """        process_emails(
            &config.output_file,
            email_cfg,
            only_call_center,
            send_exceptions,
            &config.download_path,
            config.minutes_ago,
            start_dt.as_ref(),
            &excluded_categories,
        )?;"""
new_email = """        with_retry(|| {
            process_emails(
                &config.output_file,
                email_cfg,
                only_call_center,
                send_exceptions,
                &config.download_path,
                config.minutes_ago,
                start_dt.as_ref(),
                &excluded_categories,
            )
        })?;"""
code = code.replace(old_email, new_email)

with open("src/tasker/csv_task/mod.rs", "w") as f:
    f.write(code)

# 2. DashboardUpdater
with open("src/tasker/dashboard_updater.rs", "r") as f:
    code = f.read()

code = code.replace("use anyhow::{Context, Result};", "use anyhow::{Context, Result};\nuse crate::tasker::utils::with_retry;")

old_dl2 = """    // 1. Download
    let csv_files = download_reports(config)?;"""
new_dl2 = """    // 1. Download
    let csv_files = with_retry(|| download_reports(config))?;"""
code = code.replace(old_dl2, new_dl2)

old_write2 = """    // 4. Update the dashboards (both overall and optional exception dashboard)
    update_dashboard(&config.dashboard_file, &output_rows, config.indentation_spaces)?;"""
new_write2 = """    // 4. Update the dashboards (both overall and optional exception dashboard)
    with_retry(|| update_dashboard(&config.dashboard_file, &output_rows, config.indentation_spaces))?;"""
code = code.replace(old_write2, new_write2)

old_email2 = """    if let Some(to) = &config.email_to {
        if !to.trim().is_empty() {
            send_dashboard_email(config, &config.dashboard_file, to)?;
        }
    }"""
new_email2 = """    if let Some(to) = &config.email_to {
        if !to.trim().is_empty() {
            with_retry(|| send_dashboard_email(config, &config.dashboard_file, to))?;
        }
    }"""
code = code.replace(old_email2, new_email2)

with open("src/tasker/dashboard_updater.rs", "w") as f:
    f.write(code)

# 3. DepartmentSplit
with open("src/tasker/department_split.rs", "r") as f:
    code = f.read()

code = code.replace("use anyhow::{Context, Result};", "use anyhow::{Context, Result};\nuse crate::tasker::utils::with_retry;")

old_dir = """        std::fs::create_dir_all(&config.output_dir).with_context(|| {
            format!("Failed to create output directory: {}", config.output_dir)
        })?;"""
new_dir = """        with_retry(|| -> Result<()> {
            std::fs::create_dir_all(&config.output_dir).with_context(|| {
                format!("Failed to create output directory: {}", config.output_dir)
            })?;
            Ok(())
        })?;"""
code = code.replace(old_dir, new_dir)

old_run = """        run_powershell(&script).with_context(|| {
            format!(
                "Failed to run PowerShell script for department: {}",
                department
            )
        })?;"""
new_run = """        with_retry(|| {
            run_powershell(&script).with_context(|| {
                format!(
                    "Failed to run PowerShell script for department: {}",
                    department
                )
            })
        })?;"""
code = code.replace(old_run, new_run)

with open("src/tasker/department_split.rs", "w") as f:
    f.write(code)

# 4. OpdAnalysis
with open("src/tasker/opd_task/mod.rs", "r") as f:
    code = f.read()

code = code.replace("use anyhow::{Context, Result};", "use anyhow::{Context, Result};\nuse crate::tasker::utils::with_retry;")

old_fetch = """    let (downloaded_files, _reports) = crate::tasker::opd_task::download::fetch_reports(config)?;"""
new_fetch = """    let (downloaded_files, _reports) = with_retry(|| crate::tasker::opd_task::download::fetch_reports(config))?;"""
code = code.replace(old_fetch, new_fetch)

old_send = """    powershell_email::send_opd_email(config)?;"""
new_send = """    with_retry(|| powershell_email::send_opd_email(config))?;"""
code = code.replace(old_send, new_send)

with open("src/tasker/opd_task/mod.rs", "w") as f:
    f.write(code)
