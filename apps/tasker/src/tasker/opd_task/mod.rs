pub mod csv_export;
pub mod csv_history;
pub mod data_extraction;
pub mod file_discovery;
pub mod merging;
pub mod models;
pub mod powershell_email;

use crate::tasker::config::OpdAnalysisConfig;
use anyhow::Result;
use tracing::info;

pub fn run(config: &OpdAnalysisConfig) -> Result<()> {
    let download_dir_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.download_path);
    let cus_input_path = crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.cus_input);
    let cus_file_path = crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.cus_file);

    info!("Running OpdAnalysis for folder: {:?}", download_dir_path);

    // 1. Read existing CUS and find latest hour
    let history = csv_history::read_history(&cus_input_path)?;

    let process_from = history
        .latest_archived_dt
        .map(|dt| dt + chrono::Duration::hours(1));
    info!(
        "Latest archived hour: {:?}. Processing from: {:?}",
        history.latest_archived_dt, process_from
    );

    // 2. Scan and filter files
    let new_files = file_discovery::discover_new_files(&download_dir_path, process_from);

    // 3. Process new files
    let new_data = data_extraction::extract_new_data(config, new_files);

    if new_data.is_empty() {
        info!("No new data to append.");
        return Ok(());
    }

    // 4. Merge Data
    let merge_ctx = merging::merge_data(
        history.records,
        &history.headers,
        history.hour_columns,
        new_data,
    );

    // 5. Write out
    csv_export::export_csv(&cus_file_path, merge_ctx)?;

    // 6. Generate Image and Email using PowerShell
    powershell_email::generate_and_email_image(&cus_file_path, config)?;

    Ok(())
}
