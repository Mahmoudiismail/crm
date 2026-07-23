use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_crm_startup_missing_config_fails() {
    let output = Command::new(env!("CARGO_BIN_EXE_crm"))
        .arg("--config")
        .arg("/this/path/does/not/exist/and/cannot/be/created.json")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Failed to load application configuration")
            || stderr.contains("Failed to write config")
    );
}

#[test]
fn test_crm_startup_invalid_json() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("invalid.json");
    fs::write(&config_path, "invalid json").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_crm"))
        .arg("--config")
        .arg(config_path)
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to parse config file"));
}

#[test]
fn test_crm_startup_invalid_cli_arg() {
    let output = Command::new(env!("CARGO_BIN_EXE_crm"))
        .arg("--unknown-arg")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error: unexpected argument '--unknown-arg'"));
}
