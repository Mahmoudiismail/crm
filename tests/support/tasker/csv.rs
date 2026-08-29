use tempfile::NamedTempFile;

pub struct TestDataset {
    pub users_file: NamedTempFile,
    pub assignments_file: NamedTempFile,
    pub download_dir: tempfile::TempDir,
    pub output_file: NamedTempFile,
    #[allow(dead_code)]
    pub leads_file: NamedTempFile,
    pub teams_file: NamedTempFile,
    pub config_json: String,
}

pub fn setup_test_dataset() -> TestDataset {
    let users_file = NamedTempFile::new().unwrap();
    let assignments_file = NamedTempFile::new().unwrap();
    let download_dir = tempfile::tempdir().unwrap();
    let output_file = NamedTempFile::new().unwrap();
    let leads_file = NamedTempFile::new().unwrap();
    let teams_file = NamedTempFile::new().unwrap();

    let agents_csv = std::fs::read_to_string("TestingDownloads/users.csv").unwrap();
    std::fs::write(users_file.path(), agents_csv).unwrap();

    let assignment_csv =
        std::fs::read_to_string("TestingDownloads/assignement settings.csv").unwrap();
    std::fs::write(assignments_file.path(), assignment_csv).unwrap();

    std::fs::copy(
        "TestingDownloads/ticket_report_1783634497568.csv",
        download_dir.path().join("ticket_report_1783634497568.csv"),
    )
    .unwrap();
    std::fs::copy(
        "TestingDownloads/ticket_report_1783634532999.csv",
        download_dir.path().join("ticket_report_1783634532999.csv"),
    )
    .unwrap();
    std::fs::copy(
        "TestingDownloads/ticket_report_1783634535708.csv",
        download_dir.path().join("ticket_report_1783634535708.csv"),
    )
    .unwrap();

    let leads_bytes = std::fs::read("TestingDownloads/lead_report_1783627642439.csv").unwrap();
    let leads_csv = String::from_utf8_lossy(&leads_bytes);
    std::fs::write(leads_file.path(), leads_csv.as_bytes()).unwrap();
    std::fs::copy(
        leads_file.path(),
        download_dir.path().join("lead_report_1783627642439.csv"),
    )
    .unwrap();

    let config_json = std::fs::read_to_string("TestingDownloads/tasker_config.json").unwrap();
    {
        let mut teams_wtr = csv::Writer::from_writer(teams_file.as_file());
        teams_wtr
            .write_record(["Team Name", "Receiver Name", "To Emails", "CC"])
            .unwrap();
        teams_wtr
            .write_record([
                "Incomplete Reservation",
                "Incomplete Reservation Team",
                "inc@example.com",
                "cc@example.com",
            ])
            .unwrap();
        teams_wtr
            .write_record([
                "PRE-AUTHORIZATION",
                "Pre-Auth Team",
                "preauth@example.com",
                "",
            ])
            .unwrap();
        teams_wtr
            .write_record(["Call Center", "Call Center Team", "cc@example.com", ""])
            .unwrap();
        teams_wtr.flush().unwrap();
    }

    TestDataset {
        users_file,
        assignments_file,
        download_dir,
        output_file,
        leads_file,
        teams_file,
        config_json,
    }
}
