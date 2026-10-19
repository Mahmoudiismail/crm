use anyhow::{Context, Result};
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

const OUTLOOK_HELPER_SCRIPT: &str = r#"
function Get-OutlookSenderAddress($Item) {
    $senderAddress = ""
    try { $senderAddress = [string]$Item.SenderEmailAddress } catch {}
    if ($Item.SenderEmailType -eq "EX") {
        try {
            $exchangeUser = $Item.Sender.GetExchangeUser()
            if ($exchangeUser -and $exchangeUser.PrimarySmtpAddress) { $senderAddress = [string]$exchangeUser.PrimarySmtpAddress }
        } catch {}
        if ([string]::IsNullOrWhiteSpace($senderAddress) -or $senderAddress.IndexOf("@") -lt 0) {
            try { $senderAddress = [string]$Item.PropertyAccessor.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x39FE001E") } catch {}
        }
    }
    return $senderAddress
}

function Find-OutlookOriginalMessage($Folder, [string]$SortProperty, [string]$SenderAccount, [string]$SubjectPrefix) {
    $items = $Folder.Items
    $items.Sort($SortProperty, $true)
    foreach ($item in $items) {
        if (-not $item.Subject -or -not $item.Subject.StartsWith($SubjectPrefix, [System.StringComparison]::OrdinalIgnoreCase)) { continue }
        $senderAddress = Get-OutlookSenderAddress $item
        if ([string]::Equals($senderAddress.Trim(), $SenderAccount.Trim(), [System.StringComparison]::OrdinalIgnoreCase)) { return $item }
    }
    return $null
}

function Get-OutlookHtmlFragment([string]$Html) {
    $bodyStart = $Html.IndexOf("<body", [System.StringComparison]::OrdinalIgnoreCase)
    if ($bodyStart -lt 0) { return $Html }
    $bodyTagEnd = $Html.IndexOf(">", $bodyStart)
    if ($bodyTagEnd -lt 0) { return $Html }
    $bodyEnd = $Html.LastIndexOf("</body>", [System.StringComparison]::OrdinalIgnoreCase)
    if ($bodyEnd -lt $bodyTagEnd) { $bodyEnd = $Html.Length }
    return $Html.Substring($bodyTagEnd + 1, $bodyEnd - $bodyTagEnd - 1)
}

function Send-OutlookMail {
    param(
        [ValidateSet("New", "ReplyAll")][string]$Mode = "New",
        [string]$To = "", [string]$Cc = "", [string]$Subject = "",
        [string]$HtmlBodyPath = "", [string[]]$AttachmentPaths = @(),
        [ValidateSet("Send()", "Send", "Display")][string]$DisplayOrSend = "Send()",
        [string]$BodyMode = "Replace", [string]$SenderAccount = "",
        [string]$SubjectPrefix = "", [string]$AdditionalTo = "", [string]$RemoveCc = "",
        [switch]$SendAtNextSevenAm
    )
    $ErrorActionPreference = "Stop"
    $outlook = New-Object -ComObject Outlook.Application
    if ($Mode -eq "ReplyAll") {
        if ([string]::IsNullOrWhiteSpace($SenderAccount) -or [string]::IsNullOrWhiteSpace($SubjectPrefix)) { throw "ReplyAll requires SenderAccount and SubjectPrefix." }
        $namespace = $outlook.GetNamespace("MAPI")
        $mail = Find-OutlookOriginalMessage ($namespace.GetDefaultFolder(6)) "[ReceivedTime]" $SenderAccount $SubjectPrefix
        if (-not $mail) { $mail = Find-OutlookOriginalMessage ($namespace.GetDefaultFolder(5)) "[SentOn]" $SenderAccount $SubjectPrefix }
        if (-not $mail) { throw "Original message with subject prefix '$SubjectPrefix' not found in Inbox or Sent Items of '$SenderAccount'." }
        $mail = $mail.ReplyAll()
    } else {
        $mail = $outlook.CreateItem(0)
        if ($To) { $mail.To = $To }
        if ($Cc) { $mail.CC = $Cc }
    }
    if ($AdditionalTo -and $Mode -eq "ReplyAll") {
        $currentTo = [string]$mail.To
        if ($currentTo.IndexOf($AdditionalTo, [System.StringComparison]::OrdinalIgnoreCase) -lt 0) {
            if ([string]::IsNullOrWhiteSpace($currentTo)) { $mail.To = $AdditionalTo }
            else { $mail.To = $currentTo.TrimEnd(';', ' ', ',') + "; " + $AdditionalTo }
        }
    }
    if ($RemoveCc -and $Mode -eq "ReplyAll") {
        $ccRecipients = @(([string]$mail.CC -split ';') | ForEach-Object { $_.Trim() } | Where-Object { $_ -and $_.IndexOf($RemoveCc, [System.StringComparison]::OrdinalIgnoreCase) -lt 0 })
        $mail.CC = $ccRecipients -join "; "
    }
    if ($Subject) { $mail.Subject = $Subject }
    if ($HtmlBodyPath) {
        if (-not (Test-Path -LiteralPath $HtmlBodyPath -PathType Leaf)) { throw "Email HTML body file not found: $HtmlBodyPath" }
        $htmlBody = Get-Content -LiteralPath $HtmlBodyPath -Raw -Encoding UTF8
        if ($BodyMode -eq "Prepend" -and $Mode -eq "ReplyAll") {
            $fragment = Get-OutlookHtmlFragment $htmlBody
            $existingHtml = [string]$mail.HTMLBody
            $bodyStart = $existingHtml.IndexOf("<body", [System.StringComparison]::OrdinalIgnoreCase)
            $bodyTagEnd = if ($bodyStart -ge 0) { $existingHtml.IndexOf(">", $bodyStart) } else { -1 }
            if ($bodyTagEnd -ge 0) { $mail.HTMLBody = $existingHtml.Insert($bodyTagEnd + 1, $fragment) }
            else { $mail.HTMLBody = $fragment + $existingHtml }
        } else { $mail.HTMLBody = $htmlBody }
    }
    foreach ($attachmentPath in $AttachmentPaths) {
        if ([string]::IsNullOrWhiteSpace($attachmentPath)) { continue }
        if (-not (Test-Path -LiteralPath $attachmentPath -PathType Leaf)) { throw "Email attachment not found: $attachmentPath" }
        $mail.Attachments.Add([System.IO.Path]::GetFullPath($attachmentPath)) | Out-Null
    }
    if ($SendAtNextSevenAm) {
        $now = Get-Date
        $targetTime = $now.Date.AddHours(7)
        if ($now -ge $targetTime) { $targetTime = $targetTime.AddDays(1) }
        $mail.DeferredDeliveryTime = $targetTime
    }
    if ($DisplayOrSend -eq "Display") { $mail.Display() } else { $mail.Send() }
}
"#;

const OUTLOOK_WRAPPER_TEMPLATE: &str = r#"
param(
    [string]$OutlookHelperPath, [string]$Mode, [string]$To, [string]$Cc,
    [string]$Subject, [string]$HtmlBodyPath, [string]$AttachmentListPath,
    [string]$DisplayOrSend, [string]$BodyMode,
    [string]$SenderAccount, [string]$SubjectPrefix, [string]$AdditionalTo,
    [string]$RemoveCc, [string]$SendAtNextSevenAm
)
try {
    $ErrorActionPreference = "Stop"
    . $OutlookHelperPath
    $attachments = @()
    if ($AttachmentListPath) {
        if (-not (Test-Path -LiteralPath $AttachmentListPath -PathType Leaf)) { throw "Attachment list file not found: $AttachmentListPath" }
        $attachments = @(Get-Content -LiteralPath $AttachmentListPath -Encoding UTF8 | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    }
    $sendAtSeven = $SendAtNextSevenAm -eq "true"
    Send-OutlookMail -Mode $Mode -To $To -Cc $Cc -Subject $Subject -HtmlBodyPath $HtmlBodyPath -AttachmentPaths $attachments -DisplayOrSend $DisplayOrSend -BodyMode $BodyMode -SenderAccount $SenderAccount -SubjectPrefix $SubjectPrefix -AdditionalTo $AdditionalTo -RemoveCc $RemoveCc -SendAtNextSevenAm:$sendAtSeven
} catch {
    Write-Error "Outlook operation failed: $_"
    exit 1
}
"#;

#[derive(Clone, Copy)]
struct OutlookMailRequest<'a> {
    mode: &'a str,
    to: &'a str,
    cc: &'a str,
    subject: &'a str,
    html_body: Option<&'a str>,
    attachment_paths: &'a [&'a str],
    display_or_send: &'a str,
    body_mode: &'a str,
    sender_account: &'a str,
    subject_prefix: &'a str,
    additional_to: &'a str,
    remove_cc: &'a str,
    send_at_next_seven_am: bool,
}

pub fn get_outlook_helper_path() -> Result<PathBuf> {
    crate::tasker::script_manager::ScriptManager::new().get_or_create_script(
        "_shared",
        "outlook_mail.ps1",
        OUTLOOK_HELPER_SCRIPT,
    )
}

pub fn send_email(
    to: &str,
    cc: &str,
    subject: &str,
    html_body: &str,
    attachment_path: Option<&str>,
    leads_path: Option<&str>,
    display_or_send: &str,
) -> Result<()> {
    let attachments: Vec<&str> = [attachment_path, leads_path]
        .into_iter()
        .flatten()
        .collect();
    send_email_with_attachments(to, cc, subject, html_body, &attachments, display_or_send)
}

pub fn send_email_with_attachments(
    to: &str,
    cc: &str,
    subject: &str,
    html_body: &str,
    attachment_paths: &[&str],
    display_or_send: &str,
) -> Result<()> {
    run_outlook_mail(OutlookMailRequest {
        mode: "New",
        to,
        cc,
        subject,
        html_body: Some(html_body),
        attachment_paths,
        display_or_send,
        body_mode: "Replace",
        sender_account: "",
        subject_prefix: "",
        additional_to: "",
        remove_cc: "",
        send_at_next_seven_am: false,
    })
}

pub fn reply_all_email(
    sender_account: &str,
    subject_prefix: &str,
    subject: &str,
    html_body: &str,
    additional_to: &str,
    remove_cc: &str,
    send_at_next_seven_am: bool,
) -> Result<()> {
    run_outlook_mail(OutlookMailRequest {
        mode: "ReplyAll",
        to: "",
        cc: "",
        subject,
        html_body: Some(html_body),
        attachment_paths: &[],
        display_or_send: "Send()",
        body_mode: "Prepend",
        sender_account,
        subject_prefix,
        additional_to,
        remove_cc,
        send_at_next_seven_am,
    })
}

fn run_outlook_mail(request: OutlookMailRequest<'_>) -> Result<()> {
    let manager = crate::tasker::script_manager::ScriptManager::new();
    let helper_path = get_outlook_helper_path()?;

    // Keep potentially large email HTML out of the Windows command line.
    let html_file: Option<NamedTempFile> = request
        .html_body
        .map(|body| {
            let mut file =
                NamedTempFile::new().context("Failed to create temporary email body file")?;
            file.write_all(body.as_bytes())
                .context("Failed to write temporary email body file")?;
            file.flush()
                .context("Failed to flush temporary email body file")?;
            Ok::<_, anyhow::Error>(file)
        })
        .transpose()?;
    let html_path = html_file
        .as_ref()
        .map(|file| file.path().to_string_lossy().into_owned())
        .unwrap_or_default();
    let helper_path = helper_path.to_string_lossy().into_owned();
    let attachment_list_file: Option<NamedTempFile> = if request.attachment_paths.is_empty() {
        None
    } else {
        let mut file =
            NamedTempFile::new().context("Failed to create temporary attachment list file")?;
        for attachment_path in request.attachment_paths {
            writeln!(file, "{attachment_path}")
                .context("Failed to write temporary attachment list file")?;
        }
        file.flush()
            .context("Failed to flush temporary attachment list file")?;
        Some(file)
    };
    let attachment_list_path = attachment_list_file
        .as_ref()
        .map(|file| file.path().to_string_lossy().into_owned())
        .unwrap_or_default();
    let send_at_seven = if request.send_at_next_seven_am {
        "true"
    } else {
        "false"
    };
    manager.run_script_with_args(
        "Email",
        "send_email.ps1",
        OUTLOOK_WRAPPER_TEMPLATE,
        &[
            ("-OutlookHelperPath", &helper_path),
            ("-Mode", request.mode),
            ("-To", request.to),
            ("-Cc", request.cc),
            ("-Subject", request.subject),
            ("-HtmlBodyPath", &html_path),
            ("-AttachmentListPath", &attachment_list_path),
            ("-DisplayOrSend", request.display_or_send),
            ("-BodyMode", request.body_mode),
            ("-SenderAccount", request.sender_account),
            ("-SubjectPrefix", request.subject_prefix),
            ("-AdditionalTo", request.additional_to),
            ("-RemoveCc", request.remove_cc),
            ("-SendAtNextSevenAm", send_at_seven),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_outlook_function_supports_new_and_reply_all() {
        assert!(OUTLOOK_HELPER_SCRIPT.contains("function Send-OutlookMail"));
        assert!(OUTLOOK_HELPER_SCRIPT.contains(r#"[ValidateSet("New", "ReplyAll")]"#));
        assert!(OUTLOOK_HELPER_SCRIPT.contains("$mail = $mail.ReplyAll()"));
        assert!(OUTLOOK_HELPER_SCRIPT.contains("-Encoding UTF8"));
        assert!(OUTLOOK_HELPER_SCRIPT.contains("foreach ($attachmentPath in $AttachmentPaths)"));
        assert!(OUTLOOK_HELPER_SCRIPT.contains("DeferredDeliveryTime"));
        assert!(OUTLOOK_WRAPPER_TEMPLATE.contains(". $OutlookHelperPath"));
        assert!(OUTLOOK_WRAPPER_TEMPLATE.contains("$AttachmentListPath"));
        assert!(OUTLOOK_WRAPPER_TEMPLATE.contains("Get-Content -LiteralPath $AttachmentListPath"));
    }

    #[test]
    fn test_email_body_is_passed_through_file_not_command_line() {
        assert!(OUTLOOK_HELPER_SCRIPT
            .contains("Get-Content -LiteralPath $HtmlBodyPath -Raw -Encoding UTF8"));
        assert!(include_str!("outlook.rs").contains("NamedTempFile::new"));
    }
}
