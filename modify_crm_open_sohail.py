import sys
import re

content = open("src/tasker/crm_open_sohail.rs", "r").read()

# Fix `run_powershell`
old_run_powershell = """fn run_powershell(script: &str) -> Result<()> {
    let mut temp_file = tempfile::Builder::new()
        .prefix("crm_open_sohail_")
        .suffix(".ps1")
        .tempfile()?;

    temp_file.write_all(script.as_bytes())?;
    temp_file.as_file().sync_all()?;

    let (file, path) = temp_file.keep()?;
    drop(file);

    let output = std::process::Command::new("powershell")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&path)
        .output()?;

    let _ = std::fs::remove_file(&path);

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    if !stdout_str.trim().is_empty() {
        tracing::info!("PowerShell output:\n{}", stdout_str.trim());
    }
    if !stderr_str.trim().is_empty() {
        tracing::error!("PowerShell error output:\n{}", stderr_str.trim());
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
        .prefix("crm_open_sohail_")
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
                    tracing::info!("PowerShell: {}", l);
                }
            }
        }
    });

    let stderr_thread = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(l) = line {
                if !l.trim().is_empty() {
                    tracing::error!("PowerShell Error: {}", l);
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

if old_run_powershell in content:
    content = content.replace(old_run_powershell, new_run_powershell)


# Fix the email script exception handling in crm_open_sohail
old_email_ps = """    let ps_email_script = format!(
        r#"
$Outlook = New-Object -ComObject Outlook.Application
$Mail = $Outlook.CreateItem(0)
$Mail.To = "{}"
$Mail.CC = "{}"
$Mail.Subject = "{}"
$Mail.HTMLBody = '{}'
$Mail.Send()
"#,
        email_to.replace("\"", "'"),
        email_cc.replace("\"", "'"),
        subject.replace("\"", "''"),
        final_html.replace("'", "''")
    );"""

new_email_ps = """    let ps_email_script = format!(
        r#"
$ErrorActionPreference = "Stop"
try {{
    $Outlook = New-Object -ComObject Outlook.Application
    $Mail = $Outlook.CreateItem(0)
    $Mail.To = "{to}"
    $Mail.CC = "{cc}"
    $Mail.Subject = "{subject}"
    $Mail.HTMLBody = '{html_body}'
    $Mail.Send()
}} catch {{
    if ($_.Exception -is [System.Runtime.InteropServices.COMException] -and $_.Exception.Message -match "bigger than the server allows") {{
        Write-Output "Attachment/Body too large. Sending fallback email."
        $FallbackMail = $Outlook.CreateItem(0)
        $FallbackMail.To = "{to}"
        $FallbackMail.CC = "{cc}"
        $FallbackMail.Subject = "{subject}"
        $FallbackMail.HTMLBody = '<html><body><p>The dashboard file was too large to attach/send. Please find the generated file locally.</p></body></html>'
        $FallbackMail.Send()
    }} else {{
        throw $_
    }}
}}
"#,
        to = email_to.replace("\"", "'"),
        cc = email_cc.replace("\"", "'"),
        subject = subject.replace("\"", "''"),
        html_body = final_html.replace("'", "''")
    );"""

if old_email_ps in content:
    content = content.replace(old_email_ps, new_email_ps)


# Fix Team Mapping headers parser to be more resilient
old_headers = """    let headers = rdr.headers()?.clone();
    let mut team_idx = None;
    let mut owner_idx = None;
    let mut email_idx = None;

    for (i, h) in headers.iter().enumerate() {
        let h_lower = h.trim().to_lowercase();
        if h_lower == "team name" || h_lower == "team" {
            team_idx = Some(i);
        } else if h_lower == "owner" || h_lower == "receiver name" {
            owner_idx = Some(i);
        } else if h_lower == "to emails" || h_lower == "email" {
            email_idx = Some(i);
        }
    }"""

new_headers = """    let headers = rdr.headers()?.clone();
    let mut team_idx = None;
    let mut owner_idx = None;
    let mut email_idx = None;

    for (i, h) in headers.iter().enumerate() {
        let h_lower = h.trim().to_lowercase();
        if h_lower == "team name" || h_lower == "team" || h_lower.contains("team") {
            if team_idx.is_none() { team_idx = Some(i); }
        } else if h_lower == "owner" || h_lower == "receiver name" || h_lower == "oul" || h_lower.contains("owner") || h_lower.contains("receiver") {
            if owner_idx.is_none() { owner_idx = Some(i); }
        } else if h_lower == "to emails" || h_lower == "email" || h_lower == "email_to" || h_lower.contains("email") {
            if email_idx.is_none() { email_idx = Some(i); }
        }
    }"""

if old_headers in content:
    content = content.replace(old_headers, new_headers)


open("src/tasker/crm_open_sohail.rs", "w").write(content)
print("Updated crm_open_sohail.rs")
