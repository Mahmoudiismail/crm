import sys

content = open("src/tasker/dashboard_updater.rs", "r").read()

# Replace run_powershell
old_run_powershell = """fn run_powershell(script: &str) -> Result<()> {
    let mut temp_file = tempfile::Builder::new()
        .prefix("dashboard_updater_")
        .suffix(".ps1")
        .tempfile()?;

    temp_file.write_all(script.as_bytes())?;
    temp_file.as_file().sync_all()?;

    // Explicitly keep the file on disk but drop the file handle
    // to avoid locking issues on Windows when PowerShell tries to read it.
    let (file, path) = temp_file.keep()?;
    drop(file);

    let status = std::process::Command::new("powershell")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&path)
        .output();

    // Always attempt to clean up the script regardless of success
    let output_result = status;
    let _ = std::fs::remove_file(&path);

    let output = output_result?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    if !stdout_str.trim().is_empty() {
        info!(
            "PowerShell output:\n{}",
            stdout_str.trim()
        );
    }
    if !stderr_str.trim().is_empty() {
        error!(
            "PowerShell error output:\n{}",
            stderr_str.trim()
        );
    }

    if !output.status.success() {
        anyhow::bail!("PowerShell script exited with status: {}", output.status);
    }

    Ok(())
}"""

new_run_powershell = """use std::io::{BufRead, BufReader};
use std::process::Stdio;

fn run_powershell(script: &str) -> Result<()> {
    let mut temp_file = tempfile::Builder::new()
        .prefix("dashboard_updater_")
        .suffix(".ps1")
        .tempfile()?;

    temp_file.write_all(script.as_bytes())?;
    temp_file.as_file().sync_all()?;

    let (file, path) = temp_file.keep()?;
    drop(file);

    let mut child = std::process::Command::new("powershell")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stdout_thread = std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line {
                if !l.trim().is_empty() {
                    info!("PowerShell: {}", l);
                }
            }
        }
    });

    let stderr_thread = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(l) = line {
                if !l.trim().is_empty() {
                    error!("PowerShell Error: {}", l);
                }
            }
        }
    });

    let status = child.wait()?;
    stdout_thread.join().unwrap();
    stderr_thread.join().unwrap();

    let _ = std::fs::remove_file(&path);

    if !status.success() {
        anyhow::bail!("PowerShell script exited with status: {}", status);
    }

    Ok(())
}"""

content = content.replace(old_run_powershell, new_run_powershell)


# Replace email dispatch block
old_email_dispatch = """        let ps_email_script = format!(
            r#"
$Outlook = New-Object -ComObject Outlook.Application
$Mail = $Outlook.CreateItem(0)
$Mail.To = "{}"
$Mail.CC = "{}"
$Mail.Subject = "CRM Tickets Dashboard"
$Mail.HTMLBody = "{}"
$Mail.Attachments.Add("{}")
$Mail.Send()
"#,
            email_to.replace("\"", "'"),
            email_cc.replace("\"", "'"),
            html_body.replace("\"", "''"),
            dashboard_path_str.replace("'", "''")
        );

        if let Err(e) = run_powershell(&ps_email_script) {
            error!("Failed to send dashboard email: {}", e);
            // Optionally, try a fallback email or bubble up
        } else {
            info!("Successfully sent dashboard email.");
            info!("Email sent");
        }"""

new_email_dispatch = """        let ps_email_script = format!(
            r#"
$ErrorActionPreference = "Stop"
try {{
    $Outlook = New-Object -ComObject Outlook.Application
    $Mail = $Outlook.CreateItem(0)
    $Mail.To = "{to}"
    $Mail.CC = "{cc}"
    $Mail.Subject = "CRM Tickets Dashboard"
    $Mail.HTMLBody = "{html_body}"
    $Mail.Attachments.Add("{attachment}")
    $Mail.Send()
}} catch {{
    if ($_.Exception -is [System.Runtime.InteropServices.COMException] -and $_.Exception.Message -match "bigger than the server allows") {{
        Write-Output "Attachment too large. Sending without attachment."
        $FallbackMail = $Outlook.CreateItem(0)
        $FallbackMail.To = "{to}"
        $FallbackMail.CC = "{cc}"
        $FallbackMail.Subject = "CRM Tickets Dashboard"
        $FallbackMail.HTMLBody = "<html><body><p>The dashboard file was too large to attach. Please find the generated file locally.</p></body></html>"
        $FallbackMail.Send()
    }} else {{
        throw $_
    }}
}}
"#,
            to = email_to.replace("\"", "'"),
            cc = email_cc.replace("\"", "'"),
            html_body = html_body.replace("\"", "''"),
            attachment = dashboard_path_str.replace("'", "''")
        );

        if let Err(e) = run_powershell(&ps_email_script) {
            error!("Failed to send dashboard email: {}", e);
            // Optionally, try a fallback email or bubble up
        } else {
            info!("Successfully sent dashboard email.");
            info!("Email sent");
        }"""

content = content.replace(old_email_dispatch, new_email_dispatch)

open("src/tasker/dashboard_updater.rs", "w").write(content)
print("Updated dashboard_updater.rs")
