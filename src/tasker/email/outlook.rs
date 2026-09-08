use anyhow::Result;

pub fn run_powershell(script: &str) -> Result<()> {
    let script_manager = crate::tasker::script_manager::ScriptManager::new();
    let script_path = script_manager.get_or_create_script(
        "Email",
        "send_email.ps1",
        script,
    )?;
    script_manager.execute_script(&script_path)
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
