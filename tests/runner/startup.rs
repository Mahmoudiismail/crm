use std::process::Command;

#[test]
fn malformed_existing_runner_config_exits_nonzero_without_replacing_it() {
    let temp_dir = tempfile::tempdir().expect("temporary directory should be available");
    let runner_name = if cfg!(target_os = "windows") {
        "runner.exe"
    } else {
        "runner"
    };
    let runner_path = temp_dir.path().join(runner_name);
    std::fs::copy(env!("CARGO_BIN_EXE_runner"), &runner_path)
        .expect("runner binary should be copied into the isolated directory");

    let config_path = temp_dir.path().join("runner_config.json");
    let malformed_config = "{ this is not valid JSON";
    std::fs::write(&config_path, malformed_config)
        .expect("malformed configuration should be written");

    let output = Command::new(&runner_path)
        .current_dir(temp_dir.path())
        .output()
        .expect("runner subprocess should start");

    assert_eq!(
        output.status.code(),
        Some(1),
        "runner must fail startup for an existing malformed config; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(&config_path).expect("config should remain present"),
        malformed_config,
        "startup must not overwrite an existing malformed config with defaults"
    );
}
