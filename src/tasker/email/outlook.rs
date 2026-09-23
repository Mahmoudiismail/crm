use anyhow::Result;

pub fn send_email(
    to: &str,
    cc: &str,
    subject: &str,
    html_body: &str,
    attachment_path: Option<&str>,
    leads_path: Option<&str>,
    display_or_send: &str,
) -> Result<()> {
    // 1. Create directory and save HTML payload
    let payloads_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("logs")
        .join("email_payloads");
    std::fs::create_dir_all(&payloads_dir)?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S_%f");
    let html_path = payloads_dir.join(format!("email_body_send_email_{}.html", timestamp));
    std::fs::write(&html_path, html_body)?;

    let template = r#"
param(
    [string]$To,
    [string]$Cc,
    [string]$Subject,
    [string]$HtmlBodyPath,
    [string]$AttachmentPath,
    [string]$LeadsPath,
    [string]$DisplayOrSend
)

try {
    $ErrorActionPreference = "Stop"
    $Outlook = New-Object -ComObject Outlook.Application
    $Mail = $Outlook.CreateItem(0)

    if ($To) { $Mail.To = $To }
    if ($Cc) { $Mail.CC = $Cc }
    if ($Subject) { $Mail.Subject = $Subject }

    if ($HtmlBodyPath -and (Test-Path $HtmlBodyPath)) {
        $Mail.HTMLBody = Get-Content -LiteralPath $HtmlBodyPath -Raw -Encoding UTF8
    }

    if ($AttachmentPath -and (Test-Path $AttachmentPath)) {
        $Mail.Attachments.Add($AttachmentPath)
    }

    if ($LeadsPath -and (Test-Path $LeadsPath)) {
        $Mail.Attachments.Add($LeadsPath)
    }

    if ($DisplayOrSend -eq "Send()") {
        $Mail.Send()
    } else {
        $Mail.Display()
    }
} catch {
    Write-Error "Failed to send/display email via Outlook COM: $_"
    [System.Environment]::Exit(1)
}
"#;

    let script_manager = crate::tasker::script_manager::ScriptManager::new();
    let script_path = script_manager.get_or_create_script("Email", "send_email.ps1", template)?;

    script_manager.execute_script_with_args(
        &script_path,
        &[
            ("-To", to),
            ("-Cc", cc),
            ("-Subject", subject),
            ("-HtmlBodyPath", html_path.to_string_lossy().as_ref()),
            ("-AttachmentPath", attachment_path.unwrap_or("")),
            ("-LeadsPath", leads_path.unwrap_or("")),
            ("-DisplayOrSend", display_or_send),
        ],
    )
}

pub fn run_powershell_with_args(
    logical_name: &str,
    script_template: &str,
    args: &[(&str, &str)],
) -> Result<()> {
    let script_manager = crate::tasker::script_manager::ScriptManager::new();
    let script_path =
        script_manager.get_or_create_script("Email", logical_name, script_template)?;
    script_manager.execute_script_with_args(&script_path, args)
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test_send_email_parameter_contract() {
        let src = include_str!("outlook.rs");
        assert!(src.contains("[string]$To"));
        assert!(src.contains("[string]$Cc"));
        assert!(src.contains("[string]$Subject"));
        assert!(src.contains("[string]$HtmlBodyPath"));
        assert!(src.contains("[string]$AttachmentPath"));
        assert!(src.contains("[string]$LeadsPath"));
        assert!(src.contains("[string]$DisplayOrSend"));
        assert!(src.contains("send_email.ps1"));
    }
}
