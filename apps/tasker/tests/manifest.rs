use std::process::Command;

#[test]
fn tasker_manifest_declares_process_plugin_api_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_tasker"))
        .arg("--manifest")
        .output()
        .expect("tasker manifest process should start");

    assert!(output.status.success());
    let manifest: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("tasker manifest should be valid JSON");
    assert_eq!(manifest["plugin_api_version"], 1);
}
