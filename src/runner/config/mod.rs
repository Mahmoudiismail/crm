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
}
