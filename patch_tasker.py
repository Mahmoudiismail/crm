import re
import glob
import os

# We need to wrap tasker phases using `with_retry`.
# The reviewer said: "The agent completely ignored the other required Tasker operations (CsvAnalysis, DashboardUpdater, DepartmentSplit, OpdAnalysis, and Email)."
# Let's patch each of them.

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

# 5. Email (Outlook Cascade Removal + with_retry if needed? Wait, Email is part of CsvAnalysis via process_emails.
# Wait, process_emails calls send_email_for_bucket.
# But for Outlook Cascade Removal, I must actually remove the cascade in src/tasker/email/client.rs. I did that before, let's verify.)
with open("src/tasker/email/client.rs", "r") as f:
    client_code = f.read()

if "Failed to send error notification email" in client_code:
    # Remove cascade
    old_cascade = """        if let Err(e) = run_powershell(&ps_script) {
            error!("Failed to send email for {}: {}", bucket_name, e);
            let err_script = format!(
                r#"
$Outlook = New-Object -ComObject Outlook.Application
$Mail = $Outlook.CreateItem(0)
$Mail.To = "{}"
$Mail.Subject = "Error generating email for {}"
$Mail.Body = "An error occurred while generating or sending the email for {}. Error: {}"
$Mail.Display()
"#,
                config.default_to_email,
                bucket_name,
                bucket_name,
                e.to_string().replace("\"", "'")
            );
            if let Err(e2) = run_powershell(&err_script) {
                error!("Failed to send error notification email: {}", e2);
            }
            anyhow::bail!(
                "PowerShell execution failed for email bucket {}: {}",
                bucket_name,
                e
            );
        }"""
    new_cascade = """        if let Err(e) = run_powershell(&ps_script) {
            error!("Failed to send email for {}: {}", bucket_name, e);
            anyhow::bail!(
                "PowerShell execution failed for email bucket {}: {}",
                bucket_name,
                e
            );
        }"""
    client_code = client_code.replace(old_cascade, new_cascade)

# Email (Outlook) might also need step-level retry!
# If process_emails loops over buckets, we can wrap send_email_for_bucket in a retry.
# Wait, I already wrapped `process_emails` entirely in CsvAnalysis. Is that enough?
# If process_emails fails on bucket 3, retrying process_emails repeats bucket 1 and 2.
# So I should wrap send_email_for_bucket IN process_emails instead!
client_code = client_code.replace("use anyhow::{Context, Result};", "use anyhow::{Context, Result};\nuse crate::tasker::utils::with_retry;")

old_send_b = """            send_email_for_bucket(
                &team_name,
                &team_bucket,
                only_call_center,
                send_exceptions,
            )?;"""
new_send_b = """            with_retry(|| {
                send_email_for_bucket(
                    &team_name,
                    &team_bucket,
                    only_call_center,
                    send_exceptions,
                )
            })?;"""
client_code = client_code.replace(old_send_b, new_send_b)

old_send_c = """        send_email_for_bucket("Call Center", &buckets.call_center, true)?;"""
new_send_c = """        with_retry(|| send_email_for_bucket("Call Center", &buckets.call_center, true))?;"""
client_code = client_code.replace(old_send_c, new_send_c)

with open("src/tasker/email/client.rs", "w") as f:
    f.write(client_code)

# Since I moved retry inside process_emails, I should remove it from CsvAnalysis wrapping `process_emails`.
with open("src/tasker/csv_task/mod.rs", "r") as f:
    code = f.read()

old_email_wrapper = """        with_retry(|| {
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
new_email_wrapper = """        process_emails(
            &config.output_file,
            email_cfg,
            only_call_center,
            send_exceptions,
            &config.download_path,
            config.minutes_ago,
            start_dt.as_ref(),
            &excluded_categories,
        )?;"""
code = code.replace(old_email_wrapper, new_email_wrapper)

with open("src/tasker/csv_task/mod.rs", "w") as f:
    f.write(code)
