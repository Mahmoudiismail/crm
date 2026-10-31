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
    let output_file = NamedTempFile::new_in(download_dir.path()).unwrap();
    let leads_file = NamedTempFile::new().unwrap();
    let teams_file = NamedTempFile::new().unwrap();

    // Synthetic, fictional fixtures keep integration tests independent of
    // private operational exports and the developer's local filesystem.
    std::fs::write(
        users_file.path(),
        "cognito_username,UserDepartmentName / Team Name\nalice,Call Center\nbob,Incomplete Reservation\npreauth_agent,PRE-AUTHORIZATION\n",
    )
    .unwrap();

    std::fs::write(
        assignments_file.path(),
        "Category,Type,Subtype,Auto agent/team assignment\nGeneral,Inquiry,Website,Call Center\nIncomplete Reservation,Booking,General,Incomplete Reservation\nPre-Authorization,Authorization,Review,PRE-AUTHORIZATION\n",
    )
    .unwrap();

    let ticket_csv = concat!(
        "Ticket Id,Assignee,Ticket Type,Ticket Sub-Type,Ticket Category,Created At,Branch,Branch Name,Status,Description\n",
        "T-1001,alice,Inquiry,Website,General,2026-04-10 10:00:00,DSFH,DSFH,Open,Fictional call center ticket\n",
        "T-1002,bob,Booking,General,Incomplete Reservation,2026-04-11 11:00:00,DSFH,DSFH,Open,Fictional exception ticket\n",
        "T-1003,alice,Inquiry,Website,General,2026-04-12 12:00:00,DSFH,DSFH,Open,Fictional duplicate-source ticket\n",
        "T-1004,preauth_agent,Authorization,Review,Pre-Authorization,2026-04-13 13:00:00,DSFH,DSFH,Open,Fictional pre-authorization ticket\n",
    );
    std::fs::write(
        download_dir.path().join("ticket_report_synthetic.csv"),
        ticket_csv,
    )
    .unwrap();

    std::fs::write(
        leads_file.path(),
        concat!(
            "Lead Id,Assignee,Lead Type,Lead Sub-Type,Lead Category,Created At,Branch,Status\n",
            "L-2001,alice,General Enquiry,Website,General,2026-04-10 10:00:00,DSFH,New\n",
            "L-2002,bob,General Enquiry,Website,General,2026-04-10 10:01:00,DSFH,New\n",
            "L-2003,,General Enquiry,Website,General,2026-04-10 10:02:00,DSFH,New\n",
        ),
    )
    .unwrap();
    std::fs::copy(
        leads_file.path(),
        download_dir.path().join("lead_report_synthetic.csv"),
    )
    .unwrap();

    {
        let mut teams_wtr = csv::Writer::from_writer(teams_file.as_file());
        teams_wtr
            .write_record(["Team Name", "Receiver Name", "To Emails", "CC"])
            .unwrap();
        teams_wtr
            .write_record([
                "Incomplete Reservation",
                "Incomplete Reservation Team",
                "inc@example.invalid",
                "cc@example.invalid",
            ])
            .unwrap();
        teams_wtr
            .write_record([
                "PRE-AUTHORIZATION",
                "Pre-Auth Team",
                "preauth@example.invalid",
                "",
            ])
            .unwrap();
        teams_wtr
            .write_record(["Call Center", "Call Center Team", "cc@example.invalid", ""])
            .unwrap();
        teams_wtr.flush().unwrap();
    }

    let config_json = serde_json::json!({
        "tasks": [{
            "type": "csv_analysis",
            "download_path": download_dir.path().to_string_lossy(),
            "users_file": users_file.path().to_string_lossy(),
            "assignment_settings_file": assignments_file.path().to_string_lossy(),
            "minutes_ago": 5256000_i64,
            "start_date": "01-Jan-2026",
            "exclude_branches": [],
            "exclude_categories": [],
            "category_exceptions": [{
                "category": "incomplete reservation",
                "branch": null,
                "team": "Incomplete Reservation"
            }],
            "output_file": output_file.path().to_string_lossy(),
            "email_config": {
                "team_mapping_file": teams_file.path().to_string_lossy(),
                "body_template_file": null,
                "initial_cc": "",
                "ending_cc": "",
                "send_emails": false,
                "default_to_email": "test@example.invalid",
                "send_per_team_all_branches": ["PRE-AUTHORIZATION"],
                "send_per_branch_branches": [],
                "send_per_team_branches": ["DSFH"],
                "send_call_center": true,
                "send_exceptions": false,
                "indentation_spaces": 4,
                "save_attachment_as_csv": true,
                "save_email_as_html": true
            }
        }]
    })
    .to_string();

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
