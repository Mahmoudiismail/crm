use crm_tool::crm::cleanup::cleanup_old_reports;
use filetime::{set_file_mtime, FileTime};
use std::fs;
use std::time::{Duration, SystemTime};
use tempfile::tempdir;

#[test]
fn test_cleanup_removes_old_files() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    // Create a target file that is older than 5 days
    let old_file = path.join("ticket_report_old.csv");
    fs::write(&old_file, "data").unwrap();

    // Create a target file that is newer than 5 days
    let new_file = path.join("ticket_report_new.csv");
    fs::write(&new_file, "data").unwrap();

    // Create a non-target file that is old
    let non_target = path.join("other_report_old.csv");
    fs::write(&non_target, "data").unwrap();

    // Create a target file that is old but not csv
    let old_txt = path.join("ticket_report_old.txt");
    fs::write(&old_txt, "data").unwrap();

    // Adjust timestamps manually for test
    let six_days_ago = SystemTime::now() - Duration::from_secs(6 * 86400);
    let mtime_old = FileTime::from_system_time(six_days_ago);

    set_file_mtime(&old_file, mtime_old).unwrap();
    set_file_mtime(&non_target, mtime_old).unwrap();
    set_file_mtime(&old_txt, mtime_old).unwrap();

    // Run cleanup for 5 days retention
    let deleted = cleanup_old_reports(path, 5).unwrap();

    // Only `ticket_report_old.csv` should be deleted
    assert_eq!(deleted, 1);
    assert!(!old_file.exists());
    assert!(new_file.exists());
    assert!(non_target.exists());
    assert!(old_txt.exists());
}

#[test]
fn test_cleanup_retention_zero_skips() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    let old_file = path.join("ticket_report_old.csv");
    fs::write(&old_file, "data").unwrap();

    let six_days_ago = SystemTime::now() - Duration::from_secs(6 * 86400);
    let mtime_old = FileTime::from_system_time(six_days_ago);
    set_file_mtime(&old_file, mtime_old).unwrap();

    // Run cleanup for 0 days retention
    let deleted = cleanup_old_reports(path, 0).unwrap();

    assert_eq!(deleted, 0);
    assert!(old_file.exists());
}
