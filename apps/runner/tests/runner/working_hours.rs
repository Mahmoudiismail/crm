use crm_tool::runner::config::{RunnerConfig, WorkingHours, WorkingHoursProfile};
use std::collections::HashMap;

#[test]
fn test_working_hours_profile_persistence() {
    let mut config = RunnerConfig::default();

    let mut days = HashMap::new();
    days.insert(
        "Monday".to_string(),
        WorkingHours {
            start: "09:00".to_string(),
            end: "17:00".to_string(),
        },
    );
    days.insert(
        "Tuesday".to_string(),
        WorkingHours {
            start: "09:00".to_string(),
            end: "17:00".to_string(),
        },
    );

    let profile = WorkingHoursProfile {
        id: "wh_profile_1".to_string(),
        name: "Standard Hours".to_string(),
        days: days.clone(),
    };

    config.working_hours_profiles.push(profile);

    // Save and load
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_working_hours_persistence.json");

    config.save(&config_path.to_string_lossy()).unwrap();
    let loaded_config = RunnerConfig::load(&config_path.to_string_lossy()).unwrap();
    let _ = std::fs::remove_file(config_path);

    assert_eq!(loaded_config.working_hours_profiles.len(), 1);
    let loaded_profile = &loaded_config.working_hours_profiles[0];
    assert_eq!(loaded_profile.id, "wh_profile_1");
    assert_eq!(loaded_profile.name, "Standard Hours");
    assert_eq!(loaded_profile.days.get("Monday").unwrap().start, "09:00");
    assert_eq!(loaded_profile.days.get("Monday").unwrap().end, "17:00");
}
