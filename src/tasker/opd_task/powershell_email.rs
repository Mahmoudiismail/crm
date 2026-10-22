use crate::tasker::config::OpdAnalysisConfig;
use anyhow::Result;
use std::path::Path;
use tracing::info;

pub fn get_opd_script_template() -> &'static str {
    include_str!("opd_analysis_email.ps1")
}

pub fn generate_powershell_script(
    _cus_file_path: &Path,
    _config: &OpdAnalysisConfig,
    _email_to: &str,
    _email_subject: &str,
) -> Result<String> {
    Ok(get_opd_script_template().to_string())
}

pub fn generate_and_email_image(cus_file_path: &Path, config: &OpdAnalysisConfig) -> Result<()> {
    if let (Some(email_to), Some(email_subject)) = (&config.email_to, &config.email_subject) {
        let template = get_opd_script_template();

        let special_col = config.special_column_name.clone();
        let date_col = config.date_column_name.clone();
        let check_current_year_str = if config.check_current_year {
            "true"
        } else {
            "false"
        };
        let cus_path_str = cus_file_path.to_string_lossy().to_string();

        let script_manager = crate::tasker::script_manager::ScriptManager::new();
        let helper_path = crate::tasker::email::outlook::get_outlook_helper_path()?;
        let helper_path_str = helper_path.to_string_lossy().into_owned();

        info!("Running PowerShell for generating and emailing image...");
        script_manager.run_script_with_args(
            "OPD Analysis",
            "opd_analysis_email.ps1",
            template,
            &[
                ("-CsvPath", &cus_path_str),
                ("-OutlookHelperPath", &helper_path_str),
                ("-EmailTo", email_to),
                ("-EmailSubject", email_subject),
                ("-SpecialCol", &special_col),
                ("-DateCol", &date_col),
                ("-CheckCurrentYear", check_current_year_str),
            ],
        )?;
    }

    Ok(())
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_opd_template_does_not_repeat_last_cell_scan() {
        let script = get_opd_script_template();
        assert_eq!(script.matches("$ws.Cells.Find(\"*\",").count(), 2);
        assert_eq!(script.matches("$exactRange = $ws.Range(").count(), 1);
    }

    #[test]
    fn test_opd_template_applies_manual_formatting_and_attaches_exported_image() {
        let script = get_opd_script_template();
        assert!(script.contains("$excel.Visible = $true"));
        assert!(script.contains("$excel.WindowState = -4140"));
        assert!(script.contains("$dayColIdx = -1"));
        assert!(script
            .contains("$exactRange.AutoFilter($dayColIdx, $todayDat, 1, [Type]::Missing, $false)"));
        assert!(script.contains("$chart.Export($imagePath, \"JPG\")"));
        assert!(script.contains(r#"Send-OutlookMail -Mode "New""#));
        assert!(script.contains("-AttachmentPaths @($imagePath)"));
        assert!(script.contains("$OutlookHelperPath"));
        assert!(!script.contains("$inspector.WordEditor"));
    }

    #[test]
    fn test_regression_copy_picture_bounds() {
        let config = OpdAnalysisConfig {
            download_path: "".to_string(),
            cus_input: "".to_string(),
            cus_file: "".to_string(),
            exclude_specialities: vec![],
            exclude_emp_names: vec![],
            exclude_depts: vec![],
            exclude_speciality_prefixes: vec![],
            email_to: Some("test@example.com".to_string()),
            email_subject: Some("Test".to_string()),
            special_column_name: "Special".to_string(),
            date_column_name: "KSA Time".to_string(),
            check_current_year: false,
        };

        let script = generate_powershell_script(
            &PathBuf::from("dummy.csv"),
            &config,
            "test@example.com",
            "Test Subject",
        )
        .unwrap();

        // The old buggy code would just have $usedRange.CopyPicture(1, 2)
        // The fix should replace it with a bounded range explicitly using $ws.Range and $exactRange
        assert!(
            script.contains("$copyRange = $ws.Range($ws.Cells.Item(1, 1), $ws.Cells.Item($lastRow, $exactRange.Columns.Count))"),
            "Script does not bound the copy range using $copyRange"
        );
        assert!(
            script.contains("$copyRange.CopyPicture(1, 2)"),
            "Script does not copy the bounded range"
        );

        // Ensure the bug (unbounded copy) is completely removed
        assert!(
            !script.contains("$usedRange.CopyPicture(1, 2)"),
            "Script still contains unbounded $usedRange.CopyPicture"
        );
    }
}

#[cfg(test)]
#[allow(unused_imports)]
mod opd_autofilter_tests {
    use super::*;

    #[test]
    fn test_regression_autofilter_visible_dropdown_hidden() {
        let src = include_str!("opd_analysis_email.ps1");

        // Verify that the code iterates through columns and preserves existing filters

        assert!(
            src.contains("Forcing filter icons to hide..."),
            "Should contain trace logging for hiding filter icons"
        );

        // Check that AutoFilterMode is not indiscriminately disabled
        let bad_mode_set = format!("{}{} = $false", "$ws.AutoFilter", "Mode");
        assert!(
            !src.contains(&bad_mode_set),
            "Should not blindly clear AutoFilterMode which destroys existing filters"
        );

        // Check for extraction and reapplication of existing criteria
        assert!(
            src.contains("$filter = $ws.AutoFilter.Filters.Item($col)"),
            "Should check existing filters"
        );
        assert!(
            src.contains("$exactRange.AutoFilter($col, $c1, $op, $c2, $false)"),
            "Should reapply exact criteria with VisibleDropDown = $false"
        );
    }
}
