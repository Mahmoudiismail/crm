use crm_tool::manifest::{AppArg, AppManifest, ArgType, ValidationError};

#[test]
fn test_manifest_integration_invalid_cases() {
    // 1. Missing name
    let manifest1 = AppManifest {
        name: "".to_string(),
        description: "Test".to_string(),
        arguments: vec![],
    };
    assert!(matches!(
        manifest1.validate(),
        Err(ValidationError::EmptyField { field: "name" })
    ));

    // 2. Duplicate argument
    let manifest2 = AppManifest {
        name: "App".to_string(),
        description: "Test".to_string(),
        arguments: vec![
            AppArg::new("--arg", ArgType::String),
            AppArg::new("--arg", ArgType::Boolean),
        ],
    };
    assert!(
        matches!(manifest2.validate(), Err(ValidationError::DuplicateValue { field: "arguments", invalid_value } ) if invalid_value == "--arg")
    );
}

#[test]
fn test_manifest_integration_valid_cases() {
    let manifest = AppManifest {
        name: "TestApp".to_string(),
        description: "My Test App".to_string(),
        arguments: vec![AppArg::new("--mode", ArgType::String).required(true)],
    };
    assert!(manifest.validate().is_ok());

    let json = serde_json::to_string(&manifest).unwrap();
    assert!(json.contains("TestApp"));
    assert!(json.contains("--mode"));
}
