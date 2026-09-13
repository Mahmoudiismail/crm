use serde::{Deserialize, Serialize};

use crate::runner::config::defaults::*;
use crate::runner::config::models::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAppSpecLegacy {
    pub app_id: String,
    #[serde(default)]
    pub args: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub period_mode: Option<PeriodMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
}

impl From<ExternalAppSpecLegacy> for ExternalAppSpec {
    fn from(legacy: ExternalAppSpecLegacy) -> Self {
        ExternalAppSpec {
            app_id: legacy.app_id,
            args: legacy.args,
            period_mode: legacy.period_mode.unwrap_or(PeriodMode::Custom),
            start_date: legacy.start_date,
            end_date: legacy.end_date,
        }
    }
}

impl From<ExternalAppSpec> for ExternalAppSpecLegacy {
    fn from(spec: ExternalAppSpec) -> Self {
        ExternalAppSpecLegacy {
            app_id: spec.app_id,
            args: spec.args,
            period_mode: Some(spec.period_mode),
            start_date: spec.start_date,
            end_date: spec.end_date,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionSpecLegacy {
    ShellCommand(ShellCommandSpec),
    ExternalApp(ExternalAppSpecLegacy),
}

impl From<ActionSpecLegacy> for ActionSpec {
    fn from(legacy: ActionSpecLegacy) -> Self {
        match legacy {
            ActionSpecLegacy::ShellCommand(spec) => ActionSpec::ShellCommand(spec),
            ActionSpecLegacy::ExternalApp(spec) => ActionSpec::ExternalApp(spec.into()),
        }
    }
}

impl From<ActionSpec> for ActionSpecLegacy {
    fn from(action: ActionSpec) -> Self {
        match action {
            ActionSpec::ShellCommand(spec) => ActionSpecLegacy::ShellCommand(spec),
            ActionSpec::ExternalApp(spec) => ActionSpecLegacy::ExternalApp(spec.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStepLegacy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub mode: ExecutionMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionSpecLegacy>,
}

impl From<TaskStepLegacy> for TaskStep {
    fn from(legacy: TaskStepLegacy) -> Self {
        TaskStep {
            name: legacy.name,
            mode: legacy.mode,
            actions: legacy.actions.into_iter().map(ActionSpec::from).collect(),
        }
    }
}

impl From<TaskStep> for TaskStepLegacy {
    fn from(step: TaskStep) -> Self {
        TaskStepLegacy {
            name: step.name,
            mode: step.mode,
            actions: step
                .actions
                .into_iter()
                .map(ActionSpecLegacy::from)
                .collect(),
        }
    }
}

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
    pub steps: Vec<TaskStepLegacy>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_run_steps: Vec<TaskStepLegacy>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period_mode: Option<PeriodMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
}

impl From<RunnerTaskLegacy> for RunnerTask {
    fn from(legacy: RunnerTaskLegacy) -> Self {
        let legacy_period_mode = legacy.period_mode.unwrap_or(PeriodMode::Custom);
        let legacy_start_date = legacy.start_date.clone();
        let legacy_end_date = legacy.end_date.clone();

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
                        let actions: Vec<ActionSpecLegacy> = commands
                            .into_iter()
                            .map(ActionSpecLegacy::ShellCommand)
                            .collect();

                        if !actions.is_empty() {
                            steps.push(TaskStepLegacy {
                                name: Some("Legacy Shell Command".to_string()),
                                mode: execution_mode,
                                actions,
                            });
                        }
                    }
                    TaskKind::ExternalApp { app_id, args } => {
                        steps.push(TaskStepLegacy {
                            name: Some("Legacy External App".to_string()),
                            mode: ExecutionMode::Sequential,
                            actions: vec![ActionSpecLegacy::ExternalApp(ExternalAppSpecLegacy {
                                app_id,
                                args,
                                period_mode: None,
                                start_date: None,
                                end_date: None,
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
                    post_actions.push(ActionSpecLegacy::ShellCommand(ShellCommandSpec {
                        command: script,
                        continue_on_error: false,
                    }));
                }
            }
            if let Some(app_id) = legacy.post_run_app_id {
                if !app_id.is_empty() {
                    post_actions.push(ActionSpecLegacy::ExternalApp(ExternalAppSpecLegacy {
                        app_id,
                        args: legacy.post_run_app_args.unwrap_or_default(),
                        period_mode: None,
                        start_date: None,
                        end_date: None,
                    }));
                }
            }
            if !post_actions.is_empty() {
                post_run_steps.push(TaskStepLegacy {
                    name: Some("Legacy Post-Run".to_string()),
                    mode: ExecutionMode::Sequential,
                    actions: post_actions,
                });
            }
        }

        let convert_step = |step_legacy: TaskStepLegacy| -> TaskStep {
            let actions = step_legacy
                .actions
                .into_iter()
                .map(|action_legacy| match action_legacy {
                    ActionSpecLegacy::ShellCommand(spec) => ActionSpec::ShellCommand(spec),
                    ActionSpecLegacy::ExternalApp(spec_legacy) => {
                        let period_mode = spec_legacy.period_mode.unwrap_or(legacy_period_mode);
                        let start_date =
                            spec_legacy.start_date.or_else(|| legacy_start_date.clone());
                        let end_date = spec_legacy.end_date.or_else(|| legacy_end_date.clone());

                        ActionSpec::ExternalApp(ExternalAppSpec {
                            app_id: spec_legacy.app_id,
                            args: spec_legacy.args,
                            period_mode,
                            start_date,
                            end_date,
                        })
                    }
                })
                .collect();

            TaskStep {
                name: step_legacy.name,
                mode: step_legacy.mode,
                actions,
            }
        };

        let converted_steps = steps.into_iter().map(convert_step).collect();
        let converted_post_steps = post_run_steps.into_iter().map(convert_step).collect();

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
            steps: converted_steps,
            post_run_steps: converted_post_steps,
        }
    }
}

impl From<RunnerTask> for RunnerTaskLegacy {
    fn from(task: RunnerTask) -> Self {
        let steps = task.steps.into_iter().map(TaskStepLegacy::from).collect();
        let post_run_steps = task
            .post_run_steps
            .into_iter()
            .map(TaskStepLegacy::from)
            .collect();

        RunnerTaskLegacy {
            id: task.id,
            name: task.name,
            enabled: task.enabled,
            repetition: task.repetition,
            frequency_seconds: task.frequency_seconds,
            next_run_at: task.next_run_at,
            schedules: task.schedules,
            steps,
            post_run_steps,
            kind: None,
            last_run_at: task.last_run_at,
            last_status: task.last_status,
            timeout_seconds: task.timeout_seconds,
            post_run_script: None,
            post_run_app_id: None,
            post_run_app_args: None,
            period_mode: None,
            start_date: None,
            end_date: None,
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
                    actions: vec![ActionSpec::ExternalApp(ExternalAppSpec {
                        app_id,
                        args,
                        period_mode: PeriodMode::Custom,
                        start_date: None,
                        end_date: None,
                    })],
                });
            }
        }
    }
}
