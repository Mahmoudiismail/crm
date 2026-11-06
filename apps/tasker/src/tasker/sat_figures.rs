use crate::tasker::config::{OpdAnalysisConfig, SatFiguresConfig};
use anyhow::{Context, Result};
use csv::WriterBuilder;
use chrono::NaiveDate;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::info;

fn resolve(path: &str) -> PathBuf {
    crate::tasker::csv_task::resolve_relative_to_exe_dir(path)
}

fn calculate_vacant_slots(total_slots: i64, slots_booked: i64) -> Result<i64> {
    if total_slots <= 0 {
        anyhow::bail!(
            "Sat Figures total slot count must be positive; got {}",
            total_slots
        );
    }
    if slots_booked < 0 {
        anyhow::bail!(
            "Sat Figures booked slot count cannot be negative; got {}",
            slots_booked
        );
    }
    total_slots
        .checked_sub(slots_booked)
        .filter(|vacant| *vacant >= 0)
        .with_context(|| {
            format!(
                "Sat Figures booked slots ({}) exceed total slots ({})",
                slots_booked, total_slots
            )
        })
}

fn is_source_report(path: &std::path::Path) -> bool {
    let name = path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or_default();
    let extension = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default();
    name.to_ascii_lowercase()
        .starts_with("opd- average patients seen no of schedule_")
        && !name.starts_with("~$")
        && ["xlsx", "xlsm", "xlsb", "xls"]
            .iter()
            .any(|supported| extension.eq_ignore_ascii_case(supported))
}

fn report_date_from_filename(path: &Path) -> Result<NaiveDate> {
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .context("Sat Figures source filename is not valid UTF-8")?;
    let base = filename.rsplit_once('.').map_or(filename, |(name, _)| name);
    let parts: Vec<&str> = base.split('_').collect();
    let date_text = parts
        .get(parts.len().saturating_sub(2))
        .context("Sat Figures filename must end with _DD-MM-YYYY_HHMMSS")?;
    NaiveDate::parse_from_str(date_text, "%d-%m-%Y")
        .with_context(|| format!("Invalid report date '{}' in filename {:?}", date_text, path))
}

/// Select the newest recently modified workbook for each report date.
fn recent_source_reports(
    download_dir: &Path,
    minutes_ago: u64,
) -> Result<Vec<(NaiveDate, PathBuf)>> {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(minutes_ago.saturating_mul(60)))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut reports: BTreeMap<NaiveDate, (SystemTime, PathBuf)> = BTreeMap::new();

    for entry in walkdir::WalkDir::new(download_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path().to_path_buf();
        if !is_source_report(&path) {
            continue;
        }
        let metadata = entry
            .metadata()
            .with_context(|| format!("Failed to read metadata for {:?}", path))?;
        // Use the filesystem modification timestamp, not the date embedded in the report name,
        // to determine whether Yasweb downloaded the file within the configured lookback.
        let modified = metadata
            .modified()
            .with_context(|| format!("Could not determine modification time for {:?}", path))?;
        if modified < cutoff {
            continue;
        }

        let date = match report_date_from_filename(&path) {
            Ok(date) => date,
            Err(error) => {
                warn!("Skipping Sat Figures workbook with an unrecognized date: {:#}", error);
                continue;
            }
        };

        let replace = reports
            .get(&date)
            .is_none_or(|(existing_modified, _)| modified > *existing_modified);
        if replace {
            reports.insert(date, (modified, path));
        }
    }

    let selected: Vec<_> = reports
        .into_iter()
        .map(|(date, (_, path))| (date, path))
        .collect();
    info!(
        "Found {} recent Sat Figures report date(s) within the last {} minute(s)",
        selected.len(),
        minutes_ago
    );
    Ok(selected)
}

fn get_email_script() -> &'static str {
    include_str!("sat_figures_email.ps1")
}

fn booking_email_body(date: chrono::NaiveDate) -> String {
    format!(
        "<html><body>{} Booking {}</body></html>",
        date.format("%a"),
        date.format("%d-%b")
    )
}

pub fn run(
    config: &SatFiguresConfig,
    opd_config: &OpdAnalysisConfig,
    start_date: Option<&str>,
) -> Result<()> {
    let download_dir = resolve(&config.download_path);
    if !download_dir.is_dir() {
        anyhow::bail!(
            "Sat Figures download directory does not exist: {:?}",
            download_dir
        );
    }

    let reports = recent_source_reports(&download_dir, config.minutes_ago)?;
    if reports.is_empty() {
        anyhow::bail!(
            "No OPD Average Patients Seen workbooks were modified within the last {} minute(s)",
            config.minutes_ago
        );
    }

    let filters = crate::tasker::opd_task::data_extraction::OpdFilters::from(opd_config);
    let mut rows = Vec::with_capacity(reports.len());
    for (date, path) in reports {
        let totals = crate::tasker::opd_task::data_extraction::extract_slot_totals(&path, &filters)
            .with_context(|| format!("Failed to calculate Sat Figures for {} from {:?}", date, path))?;
        let vacant_slots = calculate_vacant_slots(totals.total_slots, totals.slots_booked)?;
        let percentage = totals.slots_booked as f64 / totals.total_slots as f64;
        rows.push((date, totals, vacant_slots, percentage));
        info!(
            "Sat Figures source {:?}: booked={}, total={}, vacant={}, percentage={:.2}%",
            path,
            totals.slots_booked,
            totals.total_slots,
            vacant_slots,
            percentage * 100.0
        );
    }

    let output_path = resolve(&config.output_file);
    if let Some(parent) = output_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create Sat Figures output directory {:?}", parent)
        })?;
    }

    let mut writer = WriterBuilder::new()
        .from_path(&output_path)
        .with_context(|| format!("Failed to create Sat Figures CSV {:?}", output_path))?;
    writer.write_record(["Date", "Slots Booked", "Total Slots", "Vacant Slots", "%"])?;
    for (date, totals, vacant_slots, percentage) in &rows {
        writer.write_record([
            date.format("%d-%b-%Y").to_string(),
            totals.slots_booked.to_string(),
            totals.total_slots.to_string(),
            vacant_slots.to_string(),
            format!("{percentage:.10}"),
        ])?;
    }
    writer.flush()?;

    let csv_path = output_path.to_string_lossy().into_owned();
    let image_path = output_path.with_extension("png");
    let image_path_string = image_path.to_string_lossy().into_owned();
    let manager = crate::tasker::script_manager::ScriptManager::new();
    manager.run_script_with_args(
        "Sat Figures",
        "sat_figures_email.ps1",
        get_email_script(),
        &[("-CsvPath", &csv_path), ("-ImagePath", &image_path_string)],
    )?;

    // Preserve the existing booking-date email body for a single-day report only.
    let body = if rows.len() == 1 {
        match start_date {
            Some(value) => {
                let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").with_context(|| {
                    format!(
                        "Invalid Sat Figures start date '{}'; expected YYYY-MM-DD",
                        value
                    )
                })?;
                booking_email_body(date)
            }
            None => String::new(),
        }
    } else {
        String::new()
    };
    let attachment_paths = [image_path_string.as_str()];
    let send_result = crate::tasker::email::outlook::send_email_with_attachments(
        &config.email_to,
        &config.email_cc,
        &config.email_subject,
        &body,
        &attachment_paths,
        "Send()",
    );
    let _ = fs::remove_file(&image_path);
    send_result?;

    info!("Sat Figures completed successfully for {} report date(s).", rows.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_date_is_read_from_the_filename_date_segment() {
        let path = Path::new("OPD- Average Patients seen No of Schedule_06-11-2026_143015.xlsx");
        assert_eq!(
            report_date_from_filename(path).unwrap(),
            NaiveDate::from_ymd_opt(2026, 11, 6).unwrap()
        );
    }

    #[test]
    fn report_date_rejects_unrecognized_filename_dates() {
        let path = Path::new("OPD- Average Patients seen No of Schedule_not-a-date_143015.xlsx");
        assert!(report_date_from_filename(path).is_err());
    }

    #[test]
    fn vacant_slot_calculation_rejects_invalid_totals() {
        assert_eq!(calculate_vacant_slots(100, 75).unwrap(), 25);
        assert!(calculate_vacant_slots(0, 0).is_err());
        assert!(calculate_vacant_slots(-1, 0).is_err());
        assert!(calculate_vacant_slots(100, -1).is_err());
        assert!(calculate_vacant_slots(100, 101).is_err());
    }

    #[test]
    fn source_report_filter_matches_only_expected_prefix_and_extensions() {
        assert!(is_source_report(std::path::Path::new(
            "OPD- Average Patients seen No of Schedule_01.xlsx"
        )));
        assert!(is_source_report(std::path::Path::new(
            "opd- average patients seen no of schedule_02.xlsm"
        )));
        assert!(!is_source_report(std::path::Path::new(
            "~$OPD- Average Patients seen No of Schedule_01.xlsx"
        )));
        assert!(!is_source_report(std::path::Path::new("Other report.xlsx")));
        assert!(!is_source_report(std::path::Path::new(
            "OPD- Average Patients seen No of Schedule_01.csv"
        )));
    }

    #[test]
    fn image_script_uses_output_path_and_leaves_email_sending_to_shared_helper() {
        let script = get_email_script();
        assert!(script.contains("[string]$ImagePath"));
        assert!(script.contains("$bitmap.Save($ImagePath"));
        assert!(!script.contains("$mail.To ="));
        assert!(!script.contains("$mail.Send()"));
        assert!(!script.contains("cid:SatFiguresImage"));
    }

    #[test]
    fn booking_email_body_uses_the_weekday_of_the_report_date() {
        let saturday = chrono::NaiveDate::from_ymd_opt(2026, 10, 24).unwrap();
        let sunday = chrono::NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
        let monday = chrono::NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();

        assert_eq!(
            booking_email_body(saturday),
            "<html><body>Sat Booking 24-Oct</body></html>"
        );
        assert_eq!(
            booking_email_body(sunday),
            "<html><body>Sun Booking 25-Oct</body></html>"
        );
        assert_eq!(
            booking_email_body(monday),
            "<html><body>Mon Booking 26-Oct</body></html>"
        );
    }

    #[test]
    fn email_script_formats_percentage_as_percent() {
        assert!(get_email_script().contains("$number.ToString(\"P2\""));
    }
}
