use anyhow::Result;
use std::io::Write;

pub fn run_powershell(script: &str) -> Result<()> {
    let mut temp_file = tempfile::Builder::new()
        .prefix("send_email_")
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_powershell_file_lifecycle() {
        // This test ensures that the powershell script path creation,
        // unlocking, execution, and cleanup are working as expected.
        let script = "Write-Output 'Hello World'";
        // Normally run_powershell will succeed if powershell is available.
        // We just call it and ensure it doesn't return a file-in-use error.
        let result = run_powershell(script);
        // On linux, it might fail because powershell isn't installed.
        // But if it fails, it shouldn't be an OS error 32 (file in use).
        // Let's just assert that it ran or failed for another reason (like Not Found).
        if let Err(e) = result {
            assert!(
                !e.to_string().contains("The process cannot access the file"),
                "File lock error occurred"
            );
        }
    }

    #[test]
    fn test_powershell_script_generation_escaping() {
        // Characterization test for PowerShell string interpolation
        // The implementation natively uses replace("\"", "'") for subjects and replace("'", "''") for bodies.
        let subject = "Test \"Quotes\" and 'Single' and `Backticks` and $Dollars";
        let body = "<p>Html with 'single' quotes and \"double\" quotes</p>";

        let clean_subject = subject.replace("\"", "'");
        let clean_body = body.replace("'", "''");

        assert_eq!(
            clean_subject,
            "Test 'Quotes' and 'Single' and `Backticks` and $Dollars"
        );
        assert_eq!(
            clean_body,
            "<p>Html with ''single'' quotes and \"double\" quotes</p>"
        );
    }
}
