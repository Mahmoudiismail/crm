use std::path::PathBuf;

pub fn resolve_executable(configured: &str) -> PathBuf {
    let configured = configured.trim();
    let configured_name = if configured.is_empty() {
        default_crm_binary_name().to_string()
    } else {
        configured.to_string()
    };

    let configured_path = PathBuf::from(&configured_name);
    if configured_path.is_absolute() {
        return configured_path;
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let sibling = exe_dir.join(&configured_name);
            if sibling.exists() {
                return sibling;
            }

            if configured.is_empty() {
                let default_sibling = exe_dir.join(default_crm_binary_name());
                if default_sibling.exists() {
                    return default_sibling;
                }
            }
        }
    }

    configured_path
}

use crate::runner::config::{ActionSpec, RunnerConfig, RunnerTask, TaskStep};
use crate::runner::engine::errors::EngineError;

pub fn validate_config(config: &RunnerConfig) -> Result<(), EngineError> {
    if config.tasks.is_empty() {
        // Allow empty tasks, but maybe we want to validate registered apps
    }

    let mut app_ids = std::collections::HashSet::new();
    for app in &config.registered_apps {
        if !app_ids.insert(&app.id) {
            return Err(EngineError::Validation(format!(
                "Duplicate application ID found: {}",
                app.id
            )));
        }
    }

    let mut task_ids = std::collections::HashSet::new();
    for task in &config.tasks {
        if !task_ids.insert(&task.id) {
            return Err(EngineError::Validation(format!(
                "Duplicate task ID found: {}",
                task.id
            )));
        }
        validate_task(task, &app_ids)?;
    }

    Ok(())
}

fn validate_task(
    task: &RunnerTask,
    registered_apps: &std::collections::HashSet<&String>,
) -> Result<(), EngineError> {
    if task.steps.is_empty() {
        return Err(EngineError::Validation(format!(
            "Task '{}' has no steps in its pipeline",
            task.id
        )));
    }

    for (i, step) in task.steps.iter().enumerate() {
        validate_step(step, &task.id, &format!("Step {}", i + 1), registered_apps)?;
    }

    for (i, step) in task.post_run_steps.iter().enumerate() {
        validate_step(
            step,
            &task.id,
            &format!("Post-run Step {}", i + 1),
            registered_apps,
        )?;
    }

    Ok(())
}

fn validate_step(
    step: &TaskStep,
    task_id: &str,
    step_desc: &str,
    registered_apps: &std::collections::HashSet<&String>,
) -> Result<(), EngineError> {
    if step.actions.is_empty() {
        return Err(EngineError::Validation(format!(
            "Task '{}' contains an empty step ({})",
            task_id, step_desc
        )));
    }

    for (i, action) in step.actions.iter().enumerate() {
        match action {
            ActionSpec::ShellCommand(spec) => {
                if spec.command.trim().is_empty() {
                    return Err(EngineError::Validation(format!(
                        "Task '{}' {} action {} has an empty shell command",
                        task_id,
                        step_desc,
                        i + 1
                    )));
                }
            }
            ActionSpec::ExternalApp(spec) => {
                if spec.app_id.trim().is_empty() {
                    return Err(EngineError::Validation(format!(
                        "Task '{}' {} action {} has an empty app_id",
                        task_id,
                        step_desc,
                        i + 1
                    )));
                }
                if !registered_apps.contains(&spec.app_id) {
                    return Err(EngineError::Validation(format!(
                        "Task '{}' {} action {} references unknown app_id '{}'",
                        task_id,
                        step_desc,
                        i + 1,
                        spec.app_id
                    )));
                }
            }
        }
    }

    Ok(())
}

pub fn resolve_relative_to_exe_dir(path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        return p;
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            return exe_dir.join(p);
        }
    }

    p
}

fn default_crm_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "crm.exe"
    } else {
        "crm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_relative_to_exe_dir_absolute_path() {
        // Use an absolute path based on the OS
        let absolute_path = if cfg!(target_os = "windows") {
            "C:\\foo\\bar"
        } else {
            "/foo/bar"
        };
        let resolved = resolve_relative_to_exe_dir(absolute_path);
        assert_eq!(resolved, std::path::PathBuf::from(absolute_path));
    }

    #[test]
    fn test_resolve_relative_to_exe_dir_relative_path() {
        let relative_path = "config.json";
        let resolved = resolve_relative_to_exe_dir(relative_path);

        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let expected = exe_dir.join(relative_path);

        assert_eq!(resolved, expected);
    }

    #[test]
    fn test_resolve_relative_to_exe_dir_dot_path() {
        let dot_path = ".";
        let resolved = resolve_relative_to_exe_dir(dot_path);

        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let expected = exe_dir.join(dot_path);

        assert_eq!(resolved, expected);
    }
}
