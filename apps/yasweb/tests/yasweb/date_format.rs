use crm_tool::yasweb::config::YaswebConfig;

#[test]
fn test_yasweb_config_deserialization_handles_string_and_struct() {
    let json_legacy = r#"{
        "url": "https://example.com",
        "username": "u",
        "reports": {
            "r1": {
                "report_type": "Report Manager",
                "start_date_key": "FromDate",
                "end_date_key": "ToDate"
            }
        }
    }"#;

    let config: YaswebConfig = serde_json::from_str(json_legacy).unwrap();
    let r1 = config.reports.get("r1").unwrap();
    assert_eq!(r1.start_date_key.as_ref().unwrap().key, "FromDate");
    assert_eq!(r1.start_date_key.as_ref().unwrap().format, ""); // empty means it triggers auto-healing in binary

    let json_new = r#"{
        "url": "https://example.com",
        "username": "u",
        "reports": {
            "r1": {
                "report_type": "Standard Report",
                "start_date_key": { "key": "FromDate", "format": "%d-%m-%Y 00:00" },
                "end_date_key": { "key": "ToDate", "format": "%d-%m-%Y 23:59" }
            }
        }
    }"#;

    let config2: YaswebConfig = serde_json::from_str(json_new).unwrap();
    let r2 = config2.reports.get("r1").unwrap();
    assert_eq!(r2.start_date_key.as_ref().unwrap().key, "FromDate");
    assert_eq!(r2.start_date_key.as_ref().unwrap().format, "%d-%m-%Y 00:00");
}
