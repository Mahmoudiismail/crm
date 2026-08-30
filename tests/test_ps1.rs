use crm_tool::tasker::config::OpdAnalysisConfig;
use std::path::PathBuf;

#[test]
fn generate_and_save_ps1() {
    let script = crm_tool::tasker::opd_task::powershell_email::generate_powershell_script(
        &PathBuf::from("C:\\Users\\Rayacorp21\\Downloads\\runner_windows\\yasweb_windows\\downloads\\OPD\\cus_output.csv"),
        &OpdAnalysisConfig {
            download_path: "".into(),
            cus_input: "".into(),
            cus_file: "".into(),
            exclude_specialities: vec![],
            exclude_emp_names: vec![],
            exclude_depts: vec![],
            exclude_speciality_prefixes: vec![],
            email_to: Some("test@example.com".into()),
            email_subject: Some("Test".into()),
            special_column_name: "Special".into(),
            date_column_name: "KSA Time".into(),
            check_current_year: true,
        },
        "test@example.com",
        "Test Subject",
    ).unwrap();
    std::fs::write("/tmp/test_script.ps1", script).unwrap();
}
