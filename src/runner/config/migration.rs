use serde::{Deserialize, Serialize};

use crate::runner::config::defaults::*;
use crate::runner::config::models::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerTaskLegacy {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub repetition: Repetition,
    #[serde(default = "default_frequency")]
    pub frequency_seconds: u64,
    #[serde(default)]
    pub next_run_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub schedules: Vec<TaskSchedule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<TaskStep>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_run_steps: Vec<TaskStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<TaskKind>,
    #[serde(default)]
    pub last_run_at: String,
    #[serde(default)]
    pub last_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_run_script: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_run_app_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_run_app_args: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub timeout_seconds: u64,
}

impl From<RunnerTaskLegacy> for RunnerTask {
    fn from(legacy: RunnerTaskLegacy) -> Self {
        let mut steps = legacy.steps;
        let mut post_run_steps = legacy.post_run_steps;

        if steps.is_empty() {
            if let Some(kind) = legacy.kind {
                match kind {
                    TaskKind::ShellCommand { mode, commands } => {
                        let execution_mode = match mode {
                            ShellCommandMode::Sequential => ExecutionMode::Sequential,
                            ShellCommandMode::Parallel => ExecutionMode::Parallel,
                        };
                        let actions: Vec<ActionSpec> =
                            commands.into_iter().map(ActionSpec::ShellCommand).collect();

                        if !actions.is_empty() {
                            steps.push(TaskStep {
                                name: Some("Legacy Shell Command".to_string()),
                                mode: execution_mode,
                                actions,
                            });
                        }
                    }
                    TaskKind::ExternalApp { app_id, args } => {
                        steps.push(TaskStep {
                            name: Some("Legacy External App".to_string()),
                            mode: ExecutionMode::Sequential,
                            actions: vec![ActionSpec::ExternalApp(ExternalAppSpec {
                                app_id,
                                args,
                            })],
                        });
                    }
                }
            }
        }

        if post_run_steps.is_empty() {
            let mut post_actions = Vec::new();
            if let Some(script) = legacy.post_run_script {
                if !script.is_empty() {
                    post_actions.push(ActionSpec::ShellCommand(ShellCommandSpec {
                        command: script,
                        continue_on_error: false,
                    }));
                }
            }
            if let Some(app_id) = legacy.post_run_app_id {
                if !app_id.is_empty() {
                    post_actions.push(ActionSpec::ExternalApp(ExternalAppSpec {
                        app_id,
                        args: legacy.post_run_app_args.unwrap_or_default(),
                    }));
                }
            }
            if !post_actions.is_empty() {
                post_run_steps.push(TaskStep {
                    name: Some("Legacy Post-Run".to_string()),
                    mode: ExecutionMode::Sequential,
                    actions: post_actions,
                });
            }
        }

        RunnerTask {
            id: legacy.id,
            name: legacy.name,
            enabled: legacy.enabled,
            repetition: legacy.repetition,
            frequency_seconds: legacy.frequency_seconds,
            next_run_at: legacy.next_run_at,
            schedules: legacy.schedules,
            last_run_at: legacy.last_run_at,
            last_status: legacy.last_status,
            timeout_seconds: legacy.timeout_seconds,
            steps,
            post_run_steps,
        }
    }
}

impl From<RunnerTask> for RunnerTaskLegacy {
    fn from(task: RunnerTask) -> Self {
        RunnerTaskLegacy {
            id: task.id,
            name: task.name,
            enabled: task.enabled,
            repetition: task.repetition,
            frequency_seconds: task.frequency_seconds,
            next_run_at: task.next_run_at,
            schedules: task.schedules,
            steps: task.steps,
            post_run_steps: task.post_run_steps,
            kind: None,
            last_run_at: task.last_run_at,
            last_status: task.last_status,

            timeout_seconds: task.timeout_seconds,
            post_run_script: None,
            post_run_app_id: None,
            post_run_app_args: None,
        }
    }
}

impl RunnerTask {
    pub fn legacy_kind(&self) -> TaskKind {
        if let Some(step) = self.steps.first() {
            let mut commands = Vec::new();
            for action in &step.actions {
                match action {
                    ActionSpec::ShellCommand(spec) => commands.push(spec.clone()),
                    ActionSpec::ExternalApp(spec) => {
                        return TaskKind::ExternalApp {
                            app_id: spec.app_id.clone(),
                            args: spec.args.clone(),
                        };
                    }
                }
            }
            if !commands.is_empty() {
                let mode = match step.mode {
                    ExecutionMode::Sequential => ShellCommandMode::Sequential,
                    ExecutionMode::Parallel => ShellCommandMode::Parallel,
                };
                return TaskKind::ShellCommand { mode, commands };
            }
        }
        TaskKind::ShellCommand {
            mode: ShellCommandMode::Sequential,
            commands: Vec::new(),
        }
    }

    pub fn legacy_post_run_script(&self) -> String {
        if let Some(step) = self.post_run_steps.first() {
            for action in &step.actions {
                if let ActionSpec::ShellCommand(spec) = action {
                    return spec.command.clone();
                }
            }
        }
        String::new()
    }

    pub fn legacy_post_run_app_id(&self) -> String {
        if let Some(step) = self.post_run_steps.first() {
            for action in &step.actions {
                if let ActionSpec::ExternalApp(spec) = action {
                    return spec.app_id.clone();
                }
            }
        }
        String::new()
    }

    pub fn legacy_post_run_app_args(&self) -> std::collections::HashMap<String, String> {
        if let Some(step) = self.post_run_steps.first() {
            for action in &step.actions {
                if let ActionSpec::ExternalApp(spec) = action {
                    return spec.args.clone();
                }
            }
        }
        std::collections::HashMap::new()
    }

    pub fn set_legacy_kind(&mut self, kind: TaskKind) {
        self.steps.clear();
        match kind {
            TaskKind::ShellCommand { mode, commands } => {
                let execution_mode = match mode {
                    ShellCommandMode::Sequential => ExecutionMode::Sequential,
                    ShellCommandMode::Parallel => ExecutionMode::Parallel,
                };
                let actions: Vec<ActionSpec> =
                    commands.into_iter().map(ActionSpec::ShellCommand).collect();
                if !actions.is_empty() {
                    self.steps.push(TaskStep {
                        name: Some("Legacy Shell Command".to_string()),
                        mode: execution_mode,
                        actions,
                    });
                }
            }
            TaskKind::ExternalApp { app_id, args } => {
                self.steps.push(TaskStep {
                    name: Some("Legacy External App".to_string()),
                    mode: ExecutionMode::Sequential,
                    actions: vec![ActionSpec::ExternalApp(ExternalAppSpec { app_id, args })],
                });
            }
        }
    }
}
