use crm_tool::tasker::config::{TaskConfig, TaskerConfig};

#[test]
fn test_opd_analysis_config_serialization() {
    let json_data = r#"{
        "tasks": [
            {
                "type": "opd_analysis",
                "download_path": "./downloads",
                "cus_input": "./cus_input.csv",
                "cus_file": "./cus_output.csv",
                "exclude_specialities": ["General Practice"],
                "exclude_emp_names": ["John Doe"],
                "exclude_depts": ["Emergency"],
                "email_to": "admin@example.com",
                "email_subject": "OPD Report"
            }
        ]
    }"#;

    let config: TaskerConfig = serde_json::from_str(json_data).unwrap();
    assert_eq!(config.tasks.len(), 1);

    if let TaskConfig::OpdAnalysis(opd) = config.tasks.first().expect("Task list empty") {
        assert_eq!(opd.download_path, "./downloads");
        assert_eq!(opd.cus_input, "./cus_input.csv");
        assert_eq!(opd.cus_file, "./cus_output.csv");
        assert_eq!(opd.exclude_specialities, vec!["General Practice"]);
        assert_eq!(opd.exclude_emp_names, vec!["John Doe"]);
        assert_eq!(opd.exclude_depts, vec!["Emergency"]);
        assert_eq!(opd.email_to.as_deref(), Some("admin@example.com"));
        assert_eq!(opd.email_subject.as_deref(), Some("OPD Report"));
    } else {
        panic!("Expected OpdAnalysis task");
    }
}
