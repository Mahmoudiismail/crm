use crate::manifest::AppManifest;
use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

/// Resolves the absolute path to the directory containing the currently running executable.
///
/// Provides a robust anchor for resolving relative paths for configs, logs, and downloads
/// regardless of the current working directory from which the app was launched.
///
/// # Errors
/// Returns an error if the executable path cannot be determined or if it lacks a parent directory.
pub fn executable_dir() -> Result<PathBuf> {
    let exe_path = std::env::current_exe().context("Failed to get current executable path")?;
    let parent = exe_path
        .parent()
        .context("Executable path has no parent directory")?;
    Ok(parent.to_path_buf())
}

/// Resolve relative paths against the running executable's directory.
/// Absolute paths are returned unchanged; if the executable directory cannot
/// be determined, the original relative path is preserved.
pub fn resolve_relative_to_exe_dir(path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        return path;
    }

    match executable_dir() {
        Ok(exe_dir) => exe_dir.join(path),
        Err(_) => path,
    }
}

/// Initializes the global tracing subscriber with independent levels for stdout and file.
///
/// The file logger rolls dynamically and defaults to being placed in the executable directory.
///
/// # Parameters
/// - `app_name`: Used to construct the log file name (e.g., `<app_name>.log`).
/// - `stdout_level`: The minimum log level to display in the terminal.
/// - `file_level`: The minimum log level to persist to disk.
///
/// # Returns
/// A `WorkerGuard` that ensures logs are flushed when dropped. This must be held for the duration of the program.
pub fn setup_logging_with_levels(
    app_name: &str,
    stdout_level: tracing_subscriber::filter::LevelFilter,
    file_level: tracing_subscriber::filter::LevelFilter,
) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let log_dir =
        executable_dir().context("Could not determine executable directory for logging")?;
    let file_appender = tracing_appender::rolling::never(&log_dir, format!("{}.log", app_name));
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_filter(file_level);

    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(true)
        .with_thread_ids(true)
        .with_filter(stdout_level);

    if let Err(e) = tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .try_init()
    {
        eprintln!(
            "Warning: Failed to initialize logging for {}: {}",
            app_name, e
        );
    }

    Ok(guard)
}

#[allow(dead_code)]
pub(crate) fn setup_logging(app_name: &str) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    setup_logging_with_levels(
        app_name,
        tracing_subscriber::filter::LevelFilter::INFO,
        tracing_subscriber::filter::LevelFilter::INFO,
    )
}

/// Parses a string representation of a log level into a `tracing` LevelFilter.
///
/// Converts values safely (ignoring case), returning an explicit error if the level
/// string is unrecognized. Fails fast instead of silently defaulting.
///
/// # Errors
/// Returns an error if the string is not a valid log level (e.g. "TRACE", "DEBUG", "INFO", "WARN", "ERROR").
pub fn parse_log_level(level: &str) -> Result<tracing_subscriber::filter::LevelFilter> {
    match level.to_lowercase().as_str() {
        "trace" => Ok(tracing_subscriber::filter::LevelFilter::TRACE),
        "debug" => Ok(tracing_subscriber::filter::LevelFilter::DEBUG),
        "info" => Ok(tracing_subscriber::filter::LevelFilter::INFO),
        "warn" => Ok(tracing_subscriber::filter::LevelFilter::WARN),
        "error" => Ok(tracing_subscriber::filter::LevelFilter::ERROR),
        "off" => Ok(tracing_subscriber::filter::LevelFilter::OFF),
        _ => anyhow::bail!(
            "Invalid log level \"{}\". Valid values are: trace, debug, info, warn, error, off.",
            level
        ),
    }
}

#[derive(Debug)]
/// Result type for the `--manifest` CLI interception process.
pub enum InterceptResult {
    /// Indicates the application should proceed with normal execution.
    Continue,
    /// Indicates the manifest was successfully output and the application should exit natively without error.
    ExitSuccessfully,
}

/// Checks if the application was executed with the `--manifest` flag.
///
/// If present, outputs the provided `AppManifest` as JSON to standard output,
/// signaling the caller to exit successfully without running domain logic.
///
/// # Invariants
/// Library code must return `InterceptResult` rather than terminating the process directly (e.g., via `std::process::exit`),
/// passing control back to the binary's entry point for safe termination.
pub fn intercept_manifest(manifest: AppManifest) -> InterceptResult {
    for arg in std::env::args().skip(1) {
        if arg == "--manifest" {
            if let Ok(json) = serde_json::to_string(&manifest) {
                println!("{}", json);
            }
            return InterceptResult::ExitSuccessfully;
        }
    }
    InterceptResult::Continue
}

pub fn atomic_write(path: &Path, contents: &str) -> Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp_file = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("Failed to create temp file in {:?}", parent))?;
    temp_file
        .write_all(contents.as_bytes())
        .with_context(|| "Failed to write to temp file")?;
    temp_file
        .flush()
        .with_context(|| "Failed to flush temp file")?;
    // Sync to disk to ensure data is written before rename
    temp_file
        .as_file()
        .sync_all()
        .with_context(|| "Failed to sync temp file")?;
    temp_file
        .persist(path)
        .with_context(|| format!("Failed to atomically replace {:?}", path))?;
    Ok(())
}

pub fn merge_json(current: &mut serde_json::Value, default: &serde_json::Value) -> bool {
    let mut changed = false;
    if let (serde_json::Value::Object(curr_map), serde_json::Value::Object(def_map)) =
        (current, default)
    {
        for (k, v) in def_map {
            if !curr_map.contains_key(k) {
                curr_map.insert(k.clone(), v.clone());
                changed = true;
            } else {
                // Tasker task defaults are matched by type so reordering user tasks cannot
                // merge one task's defaults into an unrelated task. Other task arrays retain
                // their legacy positional behavior.
                if k == "tasks" {
                    if let (Some(curr_arr), Some(def_arr)) = (
                        curr_map.get_mut(k).and_then(|val| val.as_array_mut()),
                        v.as_array(),
                    ) {
                        if curr_arr.is_empty() {
                            *curr_arr = def_arr.clone();
                            changed = true;
                        } else {
                            for (i, def_item) in def_arr.iter().enumerate() {
                                let is_disabled_opt_in =
                                    def_item.get("enabled").and_then(serde_json::Value::as_bool)
                                        == Some(false);

                                if is_disabled_opt_in {
                                    let existing_index = def_item
                                        .get("type")
                                        .and_then(|task_type| {
                                            curr_arr
                                                .iter()
                                                .position(|task| task.get("type") == Some(task_type))
                                        });
                                    if let Some(index) = existing_index {
                                        // Fill missing defaults without changing the user's
                                        // enabled choice.
                                        changed |= merge_json(&mut curr_arr[index], def_item);
                                    } else {
                                        curr_arr.push(def_item.clone());
                                        changed = true;
                                    }
                                    continue;
                                }

                                let current_index = if let Some(default_type) = def_item.get("type") {
                                    curr_arr
                                        .iter()
                                        .position(|task| task.get("type") == Some(default_type))
                                        .or_else(|| {
                                            curr_arr.get(i).and_then(|task| {
                                                task.as_object()
                                                    .is_some_and(|object| object.is_empty())
                                                    .then_some(i)
                                            })
                                        })
                                } else {
                                    (i < curr_arr.len()).then_some(i)
                                };

                                if let Some(index) = current_index {
                                    if curr_arr[index]
                                        .as_object()
                                        .is_some_and(|object| object.is_empty())
                                    {
                                        curr_arr[index] = def_item.clone();
                                        changed = true;
                                    } else {
                                        changed |= merge_json(&mut curr_arr[index], def_item);
                                    }
                                }
                            }
                        }
                        continue;
                    }
                }

                if let Some(mut_val) = curr_map.get_mut(k) {
                    changed |= merge_json(mut_val, v);
                }
            }
        }
    }

    changed
}

/// Loads a JSON configuration from disk or creates it with defaults if missing.
///
/// This function intelligently merges the default configuration with any existing user configuration.
/// Arrays are strictly preserved, while objects are merged recursively. The finalized configuration
/// is saved back to disk automatically via an atomic write.
///
/// # Parameters
/// - `config_path`: The filesystem path where the configuration is expected.
/// - `default_config`: The fallback structure to inject missing keys.
///
/// # Returns
/// The fully initialized and merged configuration of type `T`.
pub fn load_or_create_config<T: DeserializeOwned + Serialize>(
    config_path: &Path,
    default_config: &T,
) -> Result<T> {
    if !config_path.exists() {
        let content = serde_json::to_string_pretty(default_config)
            .context("Failed to serialize default config")?;
        atomic_write(config_path, &content)?;
    }

    let content = std::fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config file at {:?}", config_path))?;

    let mut current_val: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file at {:?}", config_path))?;
    let default_val = serde_json::to_value(default_config)
        .with_context(|| "Failed to serialize default config to Value")?;

    let changed = merge_json(&mut current_val, &default_val);

    if changed {
        let updated_content = serde_json::to_string_pretty(&current_val)
            .context("Failed to serialize updated config")?;
        atomic_write(config_path, &updated_content)?;
    }

    let config: T = serde_json::from_value(current_val).with_context(|| {
        format!(
            "Failed to deserialize merged config file at {:?}",
            config_path
        )
    })?;

    Ok(config)
}

/// Resolves dynamic date variables (e.g., `today`, `yesterday`, `tomorrow`, `eomonth`)
/// into concrete `NaiveDate` values.
///
/// - `eomonth`: Computes the end of the month based on `base_date` or the current date if none provided.
///
/// # Parameters
/// - `val`: The date variable string to resolve.
/// - `base_date`: An optional reference date (useful for relative calculations like `eomonth`).
pub(crate) fn resolve_date_var(val: &str, base_date: Option<&str>) -> Result<chrono::NaiveDate> {
    use chrono::{Datelike, Local, NaiveDate};

    let base_dt = if let Some(bd) = base_date {
        parse_flexible_date_impl(bd, None)
    } else {
        None
    };
    let dt = base_dt.unwrap_or_else(|| Local::now().date_naive());

    let val_lower = val.trim().to_lowercase();
    match val_lower.as_str() {
        "today" => {
            let res = Local::now().date_naive();
            Ok(res)
        }
        "yesterday" => {
            let res =
                Local::now().date_naive() - chrono::TimeDelta::try_days(1).context("valid days")?;
            Ok(res)
        }
        "tomorrow" => {
            let res =
                Local::now().date_naive() + chrono::TimeDelta::try_days(1).context("valid days")?;
            Ok(res)
        }
        "this_month" | "beginning_of_month" => {
            let res = NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
                .context("valid first day of month")?;
            Ok(res)
        }
        "beginning_of_prev_month" | "beginning_of_previous_month" | "prev_month" => {
            let prev = if dt.month() == 1 {
                NaiveDate::from_ymd_opt(dt.year() - 1, 12, 1).context("valid prev year month")?
            } else {
                NaiveDate::from_ymd_opt(dt.year(), dt.month() - 1, 1).context("valid prev month")?
            };
            Ok(prev)
        }
        "eomonth" => {
            let next_month = if dt.month() == 12 {
                NaiveDate::from_ymd_opt(dt.year() + 1, 1, 1).context("valid next year")?
            } else {
                NaiveDate::from_ymd_opt(dt.year(), dt.month() + 1, 1).context("valid next month")?
            };
            let res = next_month.pred_opt().context("valid preceding day")?;
            Ok(res)
        }

        v if v.starts_with("next ") => {
            let parse_weekday = |s: &str| -> Option<chrono::Weekday> {
                match s.trim().to_lowercase().as_str() {
                    "next mon" | "next monday" => Some(chrono::Weekday::Mon),
                    "next tue" | "next tuesday" => Some(chrono::Weekday::Tue),
                    "next wed" | "next wednesday" => Some(chrono::Weekday::Wed),
                    "next thu" | "next thursday" => Some(chrono::Weekday::Thu),
                    "next fri" | "next friday" => Some(chrono::Weekday::Fri),
                    "next sat" | "next saturday" => Some(chrono::Weekday::Sat),
                    "next sun" | "next sunday" => Some(chrono::Weekday::Sun),
                    _ => None,
                }
            };

            let target_weekday = parse_weekday(v)
                .ok_or_else(|| anyhow::anyhow!("Invalid next weekday variable: {}", val))?;

            let dt = if let Some(bd) = base_date {
                if let Some(base_weekday) = parse_weekday(bd) {
                    if base_weekday == target_weekday {
                        let now_dt = Local::now().date_naive();
                        let current_weekday = now_dt.weekday();
                        let mut days_to_add = (target_weekday.num_days_from_monday() + 7
                            - current_weekday.num_days_from_monday())
                            % 7;
                        if days_to_add == 0 {
                            days_to_add = 7;
                        }
                        let res = now_dt
                            + chrono::TimeDelta::try_days(days_to_add as i64)
                                .context("valid days")?;
                        return Ok(res);
                    }
                }
                if let Some(base_dt) = parse_flexible_date_impl(bd, None) {
                    if base_dt.weekday() == target_weekday {
                        return Ok(base_dt);
                    }
                    base_dt
                } else {
                    Local::now().date_naive()
                }
            } else {
                Local::now().date_naive()
            };

            // Calculate next occurrence STRICTLY AFTER the base date
            let current_weekday = dt.weekday();
            let mut days_to_add = (target_weekday.num_days_from_monday() + 7
                - current_weekday.num_days_from_monday())
                % 7;
            if days_to_add == 0 {
                days_to_add = 7;
            }

            let res = dt + chrono::TimeDelta::try_days(days_to_add as i64).context("valid days")?;
            Ok(res)
        }
        _ => {
            if val_lower.chars().all(|c| c.is_alphabetic()) && val_lower != "may" {
                anyhow::bail!("Invalid date variable: {}", val);
            }
            anyhow::bail!("Not a date variable")
        }
    }
}

fn parse_flexible_date_impl(val: &str, base_date: Option<&str>) -> Option<chrono::NaiveDate> {
    use chrono::NaiveDate;
    let val = val.trim();
    if val.is_empty() {
        return None;
    }

    if let Ok(resolved) = resolve_date_var(val, base_date) {
        return Some(resolved);
    }

    // Check if it was meant to be a variable but failed validation
    if val.chars().all(|c| c.is_alphabetic()) && val.to_lowercase() != "may" {
        return None;
    }

    let formats = [
        "%Y-%m-%d",
        "%d-%m-%Y",
        "%d/%m/%Y",
        "%Y/%m/%d",
        "%d-%b-%Y",  // 01-May-2026
        "%d %b %Y",  // 01 May 2026
        "%b %d, %Y", // May 01, 2026
        "%d-%b-%y",  // 01-May-26
        "%d %b %y",  // 01 May 26
        "%b %d, %y", // May 01, 26
        "%y-%m-%d",  // 26-05-01
        "%y/%m/%d",  // 26/05/01
        "%d-%m-%y",  // 01-05-26
        "%d/%m/%y",  // 01/05/26
    ];

    for fmt in formats {
        if let Ok(dt) = NaiveDate::parse_from_str(val, fmt) {
            use chrono::Datelike;
            if fmt.contains("%Y") && dt.year() < 1000 {
                continue;
            }
            let dt = if dt.year() < 100 {
                dt.with_year(dt.year() + 2000).unwrap_or(dt)
            } else {
                dt
            };
            return Some(dt);
        }
    }

    None
}

pub fn parse_flexible_date(val: &str) -> Option<chrono::NaiveDate> {
    parse_flexible_date_impl(val, None)
}

pub(crate) fn parse_flexible_date_with_base(
    val: &str,
    base_date: Option<&str>,
) -> Option<chrono::NaiveDate> {
    parse_flexible_date_impl(val, base_date)
}

pub fn to_iso_date(val: &str) -> String {
    if let Some(dt) = parse_flexible_date(val) {
        dt.format("%Y-%m-%d").to_string()
    } else {
        val.trim().to_string()
    }
}

#[allow(dead_code)]
pub(crate) fn to_iso_date_with_base(val: &str, base_date: Option<&str>) -> String {
    if let Some(dt) = parse_flexible_date_with_base(val, base_date) {
        dt.format("%Y-%m-%d").to_string()
    } else {
        val.trim().to_string()
    }
}

pub fn replace_date_vars(val: &str, base_date: Option<&str>) -> String {
    if let Some(dt) = parse_flexible_date_with_base(val, base_date) {
        dt.format("%d-%m-%Y").to_string()
    } else {
        val.trim().to_string()
    }
}

pub(crate) fn build_csv_reader_builder() -> csv::ReaderBuilder {
    let mut builder = csv::ReaderBuilder::new();
    builder.has_headers(true);
    builder
}

pub fn build_csv_reader_from_reader<R: std::io::Read>(rdr: R) -> csv::Reader<R> {
    build_csv_reader_builder().from_reader(rdr)
}

pub(crate) fn generate_csv_diagnostic_context_from_file(
    file_path: impl AsRef<std::path::Path>,
    line_num: usize,
) -> String {
    use std::io::{BufRead, BufReader};
    let file = match std::fs::File::open(file_path.as_ref()) {
        Ok(f) => f,
        Err(_) => return String::new(),
    };
    let reader = BufReader::new(file);

    let start = if line_num > 20 { line_num - 20 } else { 1 };
    let end = line_num + 20;

    let mut ctx = String::new();
    for (i, line_result) in reader.lines().enumerate() {
        let current_line_num = i + 1;
        if current_line_num > end {
            break;
        }
        if current_line_num >= start {
            if let Ok(line) = line_result {
                if current_line_num == line_num {
                    ctx.push_str(&format!(">>> {:4} | {}\n", current_line_num, line));
                } else {
                    ctx.push_str(&format!("{:4} | {}\n", current_line_num, line));
                }
            }
        }
    }

    ctx
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    #[test]
    fn test_absolute_date_ignores_base_date_e2e() {
        use chrono::Local;
        let actual_today = Local::now().date_naive();
        let actual_yesterday = actual_today - chrono::TimeDelta::try_days(1).unwrap();
        let actual_tomorrow = actual_today + chrono::TimeDelta::try_days(1).unwrap();

        let legacy_base = Some("2026-01-01");

        // Ensure that resolve_date_var strictly ignores the explicit base_date
        // and resolves relative to the current local date.
        assert_eq!(
            resolve_date_var("today", legacy_base).unwrap(),
            actual_today,
            "Today should strictly ignore base_date"
        );

        assert_eq!(
            resolve_date_var("yesterday", legacy_base).unwrap(),
            actual_yesterday,
            "Yesterday should strictly ignore base_date"
        );

        assert_eq!(
            resolve_date_var("tomorrow", legacy_base).unwrap(),
            actual_tomorrow,
            "Tomorrow should strictly ignore base_date"
        );
    }
    #[test]
    fn test_resolve_date_var() {
        let today = Local::now().date_naive();
        let yesterday = today - chrono::TimeDelta::try_days(1).unwrap();
        let tomorrow = today + chrono::TimeDelta::try_days(1).unwrap();

        assert_eq!(resolve_date_var("today", None).unwrap(), today);
        assert_eq!(resolve_date_var("TODAY", None).unwrap(), today);
        assert_eq!(resolve_date_var("Today", None).unwrap(), today);

        assert_eq!(resolve_date_var("yesterday", None).unwrap(), yesterday);
        assert_eq!(resolve_date_var("tomorrow", None).unwrap(), tomorrow);

        // Test this_month
        let this_month_may = chrono::NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        assert_eq!(
            resolve_date_var("this_month", Some("2026-05-15")).unwrap(),
            this_month_may
        );

        // Test eomonth (31-day month)
        let eomonth_may = chrono::NaiveDate::from_ymd_opt(2026, 5, 31).unwrap();
        assert_eq!(
            resolve_date_var("eomonth", Some("2026-05-01")).unwrap(),
            eomonth_may
        );

        // Test eomonth (leap year)
        let eomonth_feb_leap = chrono::NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        assert_eq!(
            resolve_date_var("eomonth", Some("2024-02-01")).unwrap(),
            eomonth_feb_leap
        );

        // Test eomonth (non-leap year)
        let eomonth_feb_nonleap = chrono::NaiveDate::from_ymd_opt(2023, 2, 28).unwrap();
        assert_eq!(
            resolve_date_var("eomonth", Some("2023-02-15")).unwrap(),
            eomonth_feb_nonleap
        );

        // Ensure absolute keywords IGNORE base_date and always use local system date
        let base_date_str = "2026-01-01";
        assert_eq!(
            resolve_date_var("today", Some(base_date_str)).unwrap(),
            today
        );
        assert_eq!(
            resolve_date_var("yesterday", Some(base_date_str)).unwrap(),
            yesterday
        );
        assert_eq!(
            resolve_date_var("tomorrow", Some(base_date_str)).unwrap(),
            tomorrow
        );

        // Test invalid variables
        assert!(resolve_date_var("nextmonth", None).is_err());
        assert!(resolve_date_var("currentmonth", None).is_err());
        assert!(resolve_date_var("lastmonth", None).is_err());
        assert!(resolve_date_var("abc", None).is_err());
        assert!(resolve_date_var("abc123", None).is_err());
    }

    #[test]
    fn test_to_iso_date() {
        assert_eq!(to_iso_date("01-01-2026"), "2026-01-01");
        assert_eq!(to_iso_date("2026-01-01"), "2026-01-01");
        assert_eq!(to_iso_date("01/01/2026"), "2026-01-01");
        assert_eq!(to_iso_date("2026/01/01"), "2026-01-01");
        assert_eq!(to_iso_date("  01-01-2026  "), "2026-01-01");
        assert_eq!(to_iso_date("01-May-2026"), "2026-05-01");
        assert_eq!(to_iso_date("01 May 2026"), "2026-05-01");
        assert_eq!(to_iso_date("May 01, 2026"), "2026-05-01");
        assert_eq!(to_iso_date("01-May-26"), "2026-05-01");
        assert_eq!(to_iso_date("01 May 26"), "2026-05-01");
        assert_eq!(to_iso_date("May 01, 26"), "2026-05-01");
        assert_eq!(to_iso_date("26-05-01"), "2026-05-01");
        assert_eq!(to_iso_date("26/05/01"), "2026-05-01");
        assert_eq!(to_iso_date("invalid"), "invalid");
        assert_eq!(to_iso_date(""), "");
    }

    #[test]
    fn test_to_iso_date_with_base() {
        let eomonth_may = "2026-05-31";
        assert_eq!(
            to_iso_date_with_base("eomonth", Some("2026-05-01")),
            eomonth_may
        );
        assert_eq!(
            to_iso_date_with_base("invalid", Some("2026-05-01")),
            "invalid"
        );
    }

    #[test]
    fn test_resolve_date_var_dynamic_combinations() {
        use chrono::Datelike;
        // next sat + eomonth
        let _base = chrono::Local::now().date_naive();
        // Since test run can happen any day, we'll just check it parses and eomonth base is correct
        let resolved_start = resolve_date_var("next sat", None).unwrap();
        let resolved_end = resolve_date_var(
            "eomonth",
            Some(&resolved_start.format("%Y-%m-%d").to_string()),
        )
        .unwrap();
        assert!(resolved_start <= resolved_end);

        let feb_start = resolve_date_var("eomonth", Some("2026-02-15")).unwrap();
        assert_eq!(feb_start.format("%Y-%m-%d").to_string(), "2026-02-28");

        let feb_leap_start = resolve_date_var("eomonth", Some("2028-02-15")).unwrap();
        assert_eq!(feb_leap_start.format("%Y-%m-%d").to_string(), "2028-02-29");

        // Beginning of month -> eomonth
        let bom = resolve_date_var("beginning_of_month", None).unwrap();
        let eom = resolve_date_var("eomonth", Some(&bom.format("%Y-%m-%d").to_string())).unwrap();
        assert!(bom <= eom);
        assert_eq!(bom.month(), eom.month());
    }

    #[test]
    fn test_replace_date_vars() {
        let eomonth_may = "31-05-2026";
        assert_eq!(
            replace_date_vars("eomonth", Some("2026-05-01")),
            eomonth_may
        );
        assert_eq!(replace_date_vars("invalid", Some("2026-05-01")), "invalid");
    }

    #[test]
    fn test_parse_flexible_date() {
        assert!(parse_flexible_date("01-01-2026").is_some());
        assert!(parse_flexible_date("invalid").is_none());
        assert!(parse_flexible_date("").is_none());
        assert!(parse_flexible_date("   ").is_none());
    }

    #[test]
    fn test_atomic_write_success() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        assert!(atomic_write(&file_path, "content").is_ok());
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "content");
    }

    #[test]
    fn test_csv_reader_strict() {
        // Test strict reading, should fail on unequal lengths if flexible is false
        let csv_data = "col1,col2,col3\nval1,val2\n\nval3,val4,val5,val6\n\"multi\nline\",escaped\"\"quote,val";
        let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
        let records: Vec<_> = rdr.records().collect();

        assert!(!records.is_empty());
        // The first record has fewer columns than the header (2 vs 3).
        assert!(records[0].is_err());
    }

    #[test]
    fn test_merge_json_scalar_override() {
        let mut current = serde_json::json!({ "a": 1, "b": "user_value" });
        let default = serde_json::json!({ "a": 2, "b": "default_value" });
        let changed = super::merge_json(&mut current, &default);
        assert!(!changed); // No changes, user values preserved
        assert_eq!(current["a"], 1);
        assert_eq!(current["b"], "user_value");
    }

    #[test]
    fn test_merge_json_scalar_default() {
        let mut current = serde_json::json!({ "a": 1 });
        let default = serde_json::json!({ "a": 2, "b": "default_value" });
        let changed = super::merge_json(&mut current, &default);
        assert!(changed);
        assert_eq!(current["a"], 1);
        assert_eq!(current["b"], "default_value");
    }

    #[test]
    fn test_merge_json_task_autoheal() {
        let mut current = serde_json::json!({
            "tasks": [
                { "type": "task1", "a": 1 },
                {}
            ]
        });
        let default = serde_json::json!({
            "tasks": [
                { "type": "task1", "a": 2, "b": 2 },
                { "type": "task2", "c": 3 }
            ]
        });
        let changed = super::merge_json(&mut current, &default);
        assert!(changed);
        let curr_arr = current["tasks"].as_array().unwrap();
        assert_eq!(curr_arr.len(), 2);
        assert_eq!(curr_arr[0]["type"], "task1");
        assert_eq!(curr_arr[0]["a"], 1); // user override
        assert_eq!(curr_arr[0]["b"], 2); // default merge
        assert_eq!(curr_arr[1]["type"], "task2"); // full replace of {}
        assert_eq!(curr_arr[1]["c"], 3);
    }

    #[test]
    fn test_merge_json_appends_new_default_tasks_without_replacing_existing_tasks() {
        let mut current = serde_json::json!({
            "tasks": [
                { "type": "csv_analysis", "minutes_ago": 30 }
            ]
        });
        let default = serde_json::json!({
            "tasks": [
                { "type": "csv_analysis", "minutes_ago": 15, "output_file": "./results.csv" },
                { "type": "dashboard_updater" },
                { "type": "crm_open_sohail" },
                { "type": "department_split" },
                { "type": "opd_analysis" },
                { "type": "sat_figures", "enabled": false },
                { "type": "users_export", "enabled": false }
            ]
        });

        assert!(super::merge_json(&mut current, &default));
        assert_eq!(current["tasks"].as_array().unwrap().len(), 3);
        assert_eq!(current["tasks"][0]["minutes_ago"], 30);
        assert_eq!(current["tasks"][0]["output_file"], "./results.csv");
        assert_eq!(current["tasks"][1]["type"], "sat_figures");
        assert_eq!(current["tasks"][2]["type"], "users_export");
    }

    #[test]
    fn test_merge_json_array_atomicity() {
        let mut current = serde_json::json!({ "arr": [1, 2] });
        let default = serde_json::json!({ "arr": [3, 4, 5], "new_arr": [6] });
        let changed = super::merge_json(&mut current, &default);
        assert!(changed);
        assert_eq!(current["arr"].as_array().unwrap().len(), 2);
        assert_eq!(current["arr"][0], 1);
        assert_eq!(current["arr"][1], 2);
        assert_eq!(current["new_arr"].as_array().unwrap().len(), 1);
        assert_eq!(current["new_arr"][0], 6);
    }

    #[test]
    fn test_merge_json_nested_object() {
        let mut current = serde_json::json!({
            "obj": { "b": 2 }
        });
        let default = serde_json::json!({
            "obj": { "b": 3, "d": 4 }
        });
        let changed = super::merge_json(&mut current, &default);
        assert!(changed);
        assert_eq!(current["obj"]["b"], 2);
        assert_eq!(current["obj"]["d"], 4);
    }

    #[test]
    fn test_merge_json_empty_object() {
        let mut current = serde_json::json!({});
        let default = serde_json::json!({ "a": 1, "obj": {} });
        let changed = super::merge_json(&mut current, &default);
        assert!(changed);
        assert_eq!(current["a"], 1);
        assert!(current["obj"].as_object().unwrap().is_empty());
    }

    #[test]
    fn test_merge_json_missing_fields() {
        let mut current = serde_json::json!({ "x": 10 });
        let default = serde_json::json!({ "y": 20, "z": { "nested": true } });
        let changed = super::merge_json(&mut current, &default);
        assert!(changed);
        assert_eq!(current["x"], 10);
        assert_eq!(current["y"], 20);
        assert_eq!(current["z"]["nested"], true);
    }

    #[test]
    fn test_merge_json_idempotent() {
        let mut current = serde_json::json!({ "a": 1, "obj": { "b": 2 } });
        let default = serde_json::json!({ "a": 1, "obj": { "b": 2 } });
        let changed = super::merge_json(&mut current, &default);
        assert!(!changed);
        assert_eq!(current, default);
    }

    #[test]
    fn test_merge_json_no_change() {
        let mut current = serde_json::json!({
            "a": 1
        });
        let default = serde_json::json!({
            "a": 2
        });

        let changed = super::merge_json(&mut current, &default);
        assert!(!changed);
        assert_eq!(current["a"], 1);
    }

    #[test]
    fn test_executable_dir_resolves() {
        // Since tests run from within the workspace, this should succeed.
        let dir = executable_dir();
        assert!(dir.is_ok());
        let path = dir.unwrap();
        assert!(path.is_absolute(), "Expected absolute path, got {:?}", path);
    }

    #[test]
    fn test_generate_csv_diagnostic_context() {
        let lines: Vec<String> = (1..=50).map(|i| format!("Line {}", i)).collect();
        let file_content = lines.join("\n");
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test_file.csv");
        std::fs::write(&file_path, &file_content).unwrap();

        // Test middle
        let ctx = generate_csv_diagnostic_context_from_file(&file_path, 25);
        let ctx_lines: Vec<&str> = ctx.trim().split('\n').collect();
        assert_eq!(ctx_lines.len(), 41); // 20 before + 1 error + 20 after
        assert_eq!(ctx_lines[0].trim(), "5 | Line 5");
        assert_eq!(ctx_lines[20].trim(), ">>>   25 | Line 25");
        assert_eq!(ctx_lines[40].trim(), "45 | Line 45");

        // Test beginning bounds
        let ctx_early = generate_csv_diagnostic_context_from_file(&file_path, 5);
        let ctx_early_lines: Vec<&str> = ctx_early.trim().split('\n').collect();
        assert_eq!(ctx_early_lines.len(), 25);
        assert_eq!(ctx_early_lines[0].trim(), "1 | Line 1");
        assert_eq!(ctx_early_lines[4].trim(), ">>>    5 | Line 5");
        assert_eq!(ctx_early_lines[24].trim(), "25 | Line 25");

        // Test end bounds
        let ctx_late = generate_csv_diagnostic_context_from_file(&file_path, 45);
        let ctx_late_lines: Vec<&str> = ctx_late.trim().split('\n').collect();
        assert_eq!(ctx_late_lines.len(), 26);
        assert_eq!(ctx_late_lines[0].trim(), "25 | Line 25");
        assert_eq!(ctx_late_lines[20].trim(), ">>>   45 | Line 45");
        assert_eq!(ctx_late_lines[25].trim(), "50 | Line 50");
    }

    #[test]
    fn test_parse_log_level_valid() {
        use tracing_subscriber::filter::LevelFilter;

        assert_eq!(parse_log_level("trace").unwrap(), LevelFilter::TRACE);
        assert_eq!(parse_log_level("debug").unwrap(), LevelFilter::DEBUG);
        assert_eq!(parse_log_level("info").unwrap(), LevelFilter::INFO);
        assert_eq!(parse_log_level("warn").unwrap(), LevelFilter::WARN);
        assert_eq!(parse_log_level("error").unwrap(), LevelFilter::ERROR);
        assert_eq!(parse_log_level("off").unwrap(), LevelFilter::OFF);
    }

    #[test]
    fn test_parse_log_level_case_insensitive() {
        use tracing_subscriber::filter::LevelFilter;

        assert_eq!(parse_log_level("TRACE").unwrap(), LevelFilter::TRACE);
        assert_eq!(parse_log_level("Debug").unwrap(), LevelFilter::DEBUG);
        assert_eq!(parse_log_level("InFo").unwrap(), LevelFilter::INFO);
        assert_eq!(parse_log_level("WaRn").unwrap(), LevelFilter::WARN);
        assert_eq!(parse_log_level("ERROR").unwrap(), LevelFilter::ERROR);
        assert_eq!(parse_log_level("Off").unwrap(), LevelFilter::OFF);
    }

    #[test]
    fn test_parse_log_level_invalid() {
        let invalid_cases = ["invalid", "verbose", "", " info ", "123", "warning"];

        for case in invalid_cases {
            let res = parse_log_level(case);
            assert!(res.is_err(), "Expected error for input '{}'", case);
            let err_msg = res.unwrap_err().to_string();
            assert!(
                err_msg.contains(&format!("Invalid log level \"{}\"", case)),
                "Unexpected error message for '{}': {}",
                case,
                err_msg
            );
        }
    }
}

/// A guard that automatically removes a file when it goes out of scope.
/// This prevents temporary files from leaking if an early return (e.g. `?`) occurs.
#[derive(Debug)]
pub struct FileCleanupGuard {
    path: PathBuf,
}

impl FileCleanupGuard {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl Drop for FileCleanupGuard {
    fn drop(&mut self) {
        if self.path.exists() {
            if let Err(e) = std::fs::remove_file(&self.path) {
                tracing::warn!(
                    "Failed to securely delete temporary file {:?}: {}",
                    self.path,
                    e
                );
            }
        }
    }
}


#[cfg(test)]
mod configuration_merge_regression_tests {
    use super::merge_json;
    use serde_json::json;

    #[test]
    fn task_defaults_follow_task_type_when_user_reorders_tasks() {
        let defaults = json!({
            "tasks": [
                {
                    "type": "csv_analysis",
                    "enabled": true,
                    "minutes_ago": 15,
                    "email_config": {
                        "send_emails": false,
                        "default_to_email": "fallback@example.com"
                    }
                },
                {
                    "type": "dashboard_updater",
                    "enabled": true,
                    "dashboard_file": "default.xlsx"
                },
                {
                    "type": "sat_figures",
                    "enabled": false,
                    "output_file": "sat_figures.csv"
                }
            ]
        });
        let mut current = json!({
            "tasks": [
                {
                    "type": "dashboard_updater",
                    "enabled": true,
                    "dashboard_file": "custom.xlsx"
                },
                {
                    "type": "csv_analysis",
                    "enabled": true,
                    "minutes_ago": 30,
                    "email_config": {
                        "send_emails": true
                    }
                }
            ]
        });

        assert!(merge_json(&mut current, &defaults));

        assert_eq!(current["tasks"][0]["type"], "dashboard_updater");
        assert_eq!(current["tasks"][0]["dashboard_file"], "custom.xlsx");
        assert_eq!(current["tasks"][1]["type"], "csv_analysis");
        assert_eq!(current["tasks"][1]["minutes_ago"], 30);
        assert!(current["tasks"][1]["email_config"]["send_emails"]
            .as_bool()
            .unwrap_or(false));
        assert_eq!(
            current["tasks"][1]["email_config"]["default_to_email"],
            "fallback@example.com"
        );
        assert_eq!(current["tasks"][2]["type"], "sat_figures");
        assert!(!current["tasks"][2]["enabled"]
            .as_bool()
            .unwrap_or(true));
    }

    #[test]
    fn existing_opt_in_task_gets_missing_defaults_without_overwriting_enabled_choice() {
        let defaults = json!({
            "tasks": [
                {
                    "type": "users_export",
                    "enabled": false,
                    "minutes_ago": 10,
                    "output_file": "users.csv"
                }
            ]
        });
        let mut current = json!({
            "tasks": [
                {
                    "type": "users_export",
                    "enabled": true,
                    "output_file": "custom-users.csv"
                }
            ]
        });

        assert!(merge_json(&mut current, &defaults));
        assert!(current["tasks"][0]["enabled"].as_bool().unwrap_or(false));
        assert_eq!(current["tasks"][0]["minutes_ago"], 10);
        assert_eq!(current["tasks"][0]["output_file"], "custom-users.csv");
    }

    #[test]
    fn atomic_write_supports_a_relative_filename_without_parent_directory() {
        let file_name = format!("crm-atomic-write-relative-{}.json", std::process::id());
        let path = std::path::Path::new(&file_name);

        super::atomic_write(path, r#"{"saved":true}"#).unwrap();
        let saved = std::fs::read_to_string(path).unwrap();
        let _ = std::fs::remove_file(path);

        assert_eq!(saved, r#"{"saved":true}"#);
    }

    #[test]
    fn malformed_config_is_not_replaced_with_defaults() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("malformed.json");
        let malformed = r#"{"enabled":"#;
        std::fs::write(&path, malformed).unwrap();
        let defaults = json!({ "enabled": true });

        assert!(super::load_or_create_config(&path, &defaults).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), malformed);
    }

    #[test]
    fn ordinary_arrays_remain_atomic_during_merge() {
        let defaults = json!({ "exclude_branches": ["default"], "new_field": true });
        let mut current = json!({ "exclude_branches": ["custom", "other"] });

        assert!(merge_json(&mut current, &defaults));
        assert_eq!(current["exclude_branches"], json!(["custom", "other"]));
        assert!(current["new_field"].as_bool().unwrap_or(false));
    }
}
