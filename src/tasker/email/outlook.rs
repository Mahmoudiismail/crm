use anyhow::{Context, Result};
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

const OUTLOOK_HELPER_SCRIPT: &str = include_str!("outlook_mail.ps1");


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
        send_at_next_seven_am: false,
    })
}

pub fn reply_all_email(
    sender_account: &str,
    subject_prefix: &str,
    subject: &str,
    html_body: &str,
    additional_to: &str,
    send_at_next_seven_am: bool,
) -> Result<()> {
    run_outlook_mail(OutlookMailRequest {
        mode: "ReplyAll",
        to: "",
        cc: "",
        subject,
        html_body: Some(html_body),
        attachment_paths: &[],
        display_or_send: "SaveDraft",
        body_mode: "Prepend",
        sender_account,
        subject_prefix,
        additional_to,
        send_at_next_seven_am: false,
    })
}

fn run_outlook_mail(request: OutlookMailRequest<'_>) -> Result<()> {
    let manager = crate::tasker::script_manager::ScriptManager::new();
    let script_path = manager.get_or_create_script("Email", "outlook_mail.ps1", OUTLOOK_HELPER_SCRIPT)?;

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
    manager.execute_script_with_args(
        &script_path,
        &[
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
        assert!(!OUTLOOK_HELPER_SCRIPT.contains("ResolveAll()"));
        assert!(!OUTLOOK_HELPER_SCRIPT.contains("RemoveCc"));
        assert!(OUTLOOK_HELPER_SCRIPT.contains("SaveDraft"));
        assert!(OUTLOOK_HELPER_SCRIPT.contains("Write-OutlookStep"));
        assert!(OUTLOOK_HELPER_SCRIPT.contains("-Encoding UTF8"));
        assert!(OUTLOOK_HELPER_SCRIPT.contains("foreach ($attachmentPath in $AttachmentPaths)"));
        assert!(OUTLOOK_HELPER_SCRIPT.contains("DeferredDeliveryTime"));
        assert!(OUTLOOK_HELPER_SCRIPT.contains("$AttachmentListPath"));
        assert!(OUTLOOK_HELPER_SCRIPT.contains("Get-Content -LiteralPath $AttachmentListPath"));
    }

    #[test]
    fn test_email_body_is_passed_through_file_not_command_line() {
        assert!(OUTLOOK_HELPER_SCRIPT
            .contains("Get-Content -LiteralPath $HtmlBodyPath -Raw -Encoding UTF8"));
        assert!(include_str!("outlook.rs").contains("NamedTempFile::new"));
    }
}
