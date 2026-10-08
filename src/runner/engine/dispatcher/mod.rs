pub mod helpers;
pub mod lifecycle;
pub mod app_commands;
pub mod profile_commands;
pub mod schedule;
pub mod task_commands;

pub use lifecycle::{spawn_execution_manager, start_scheduler};
pub use task_commands::{create_task, delete_task, run_due_tasks, update_task};

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::info;

use crate::runner::engine::state::{ExecutionManagerCommand, RunnerCommand, RunnerStatus};
use profile_commands::{
    create_working_hours_profile, delete_working_hours_profile, update_working_hours_profile,
};
use app_commands::{
    create_registered_app, delete_registered_app, update_registered_app,
};
use task_commands::{run_all_tasks_now, run_task_by_id, set_task_enabled};

/// Internal route for RunnerCommand parsing
pub(crate) async fn mod_handle_command(
    path: &str,
    cmd: RunnerCommand,
    _status: &Arc<Mutex<RunnerStatus>>,
    exec_tx: &mpsc::Sender<ExecutionManagerCommand>,
) -> Result<()> {
    match cmd {
        RunnerCommand::RunAllNow { is_manual } => {
            run_all_tasks_now(path, _status, exec_tx, is_manual).await
        }
        RunnerCommand::RunTaskNow { task_id, is_manual } => {
            run_task_by_id(path, &task_id, _status, exec_tx, is_manual).await
        }
        RunnerCommand::SetTaskEnabled { task_id, enabled, reply } => {
            let res = set_task_enabled(path, &task_id, enabled).await.map_err(|e| e.to_string());
            let _ = reply.send(res);
            Ok(())
        }
        RunnerCommand::CreateWorkingHoursProfile { profile, reply } => {
            let res = create_working_hours_profile(path, profile).await.map_err(|e| e.to_string());
            let _ = reply.send(res);
            Ok(())
        }
        RunnerCommand::UpdateWorkingHoursProfile { profile, reply } => {
            let res = update_working_hours_profile(path, profile).await.map_err(|e| e.to_string());
            let _ = reply.send(res);
            Ok(())
        }
        RunnerCommand::DeleteWorkingHoursProfile { profile_id, reply } => {
            let res = delete_working_hours_profile(path, profile_id).await.map_err(|e| e.to_string());
            let _ = reply.send(res);
            Ok(())
        }
        RunnerCommand::CreateRegisteredApp { app, reply } => {
            let res = create_registered_app(path, app).await.map_err(|e| e.to_string());
            let _ = reply.send(res);
            Ok(())
        }
        RunnerCommand::UpdateRegisteredApp { app, reply } => {
            let res = update_registered_app(path, app).await.map_err(|e| e.to_string());
            let _ = reply.send(res);
            Ok(())
        }
        RunnerCommand::DeleteRegisteredApp { app_id, reply } => {
            let res = delete_registered_app(path, &app_id).await.map_err(|e| e.to_string());
            let _ = reply.send(res);
            Ok(())
        }
        RunnerCommand::Shutdown => {
            info!("Received Shutdown command in handle_command");
            Ok(())
        }
    }
}
