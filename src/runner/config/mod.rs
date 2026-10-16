pub mod defaults;
pub mod loader;
pub mod migration;
pub mod models;
pub mod schedule;
pub mod validation;

pub use migration::*;
pub use models::*;
pub use schedule::*;
pub use validation::*;

#[cfg(test)]
mod startup_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_runner_startup_fails_on_corrupt_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("runner.json");
        let path_str = config_path.to_str().unwrap();

        std::fs::write(path_str, b"{ broken json").unwrap();

        let res = RunnerConfig::load(path_str);
        assert!(res.is_err());
        let err_str = res.err().unwrap().to_string();
        assert!(!err_str.is_empty());
    }
    #[test]
    fn legacy_config_round_trip_preserves_schedules_and_date_argument_mappings() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("runner.json");
        let path_str = config_path.to_str().unwrap();
        let legacy_json = r#"{
            "gui_host": "127.0.0.1",
            "gui_port": 8787,
            "tasks": [{
                "id": "incomplete_Reservation",
                "name": "Incomplete Reservation",
                "enabled": true,
                "repetition": "repeat",
                "frequency_seconds": 86400,
                "next_run_at": "2026-10-14T17:00:00+00:00",
                "schedules": [{
                    "type": "daily_times",
                    "enabled": true,
                    "times": ["20:00"],
                    "next_run_at": "2026-10-14T17:00:00+00:00"
                }],
                "steps": [{
                    "name": "CRM",
                    "mode": "sequential",
                    "actions": [{
                        "type": "external_app",
                        "app_id": "CRM",
                        "args": {"--report": "incomplete_reservation"},
                        "period_mode": "custom",
                        "start_date": "2026-01-01",
                        "end_date": "today",
                        "start_date_arg": "--start-date",
                        "end_date_arg": "--end-date"
                    }]
                }],
                "post_run_steps": [],
                "last_run_at": "",
                "last_status": "",
                "timeout_seconds": 0
            }]
        }"#;
        std::fs::write(&config_path, legacy_json).unwrap();

        let loaded = RunnerConfig::load(path_str).expect("legacy config should load");
        assert_eq!(loaded.tasks.len(), 1);
        assert_eq!(loaded.tasks[0].id, "incomplete_Reservation");
        assert_eq!(loaded.tasks[0].frequency_seconds, 86400);
        assert_eq!(loaded.tasks[0].schedules.len(), 1);

        loaded.save(path_str).expect("legacy config should save");
        let reloaded = RunnerConfig::load(path_str).expect("saved config should reload");
        let ActionSpec::ExternalApp(app) = &reloaded.tasks[0].steps[0].actions[0] else {
            panic!("expected external app action");
        };
        assert_eq!(app.start_date_arg.as_deref(), Some("--start-date"));
        assert_eq!(app.end_date_arg.as_deref(), Some("--end-date"));
        assert_eq!(
            app.args.get("--report").map(String::as_str),
            Some("incomplete_reservation")
        );
        assert_eq!(reloaded.tasks[0].schedules.len(), 1);
        assert_eq!(reloaded.tasks[0].frequency_seconds, 86400);
    }
}
