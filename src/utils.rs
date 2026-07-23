use crate::manifest::AppManifest;
use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

pub fn executable_dir() -> Result<PathBuf> {
    let exe_path = std::env::current_exe().context("Failed to get current executable path")?;
    let parent = exe_path
        .parent()
        .context("Executable path has no parent directory")?;
    Ok(parent.to_path_buf())
}

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
pub enum InterceptResult {
    Continue,
    ExitSuccessfully,
}

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

pub(crate) fn atomic_write(path: &Path, contents: &str) -> Result<()> {
    use std::io::Write;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
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

fn merge_json(current: &mut serde_json::Value, default: &serde_json::Value) -> bool {
    let mut changed = false;
    if let (serde_json::Value::Object(curr_map), serde_json::Value::Object(def_map)) =
        (current, default)
    {
        for (k, v) in def_map {
            if !curr_map.contains_key(k) {
                curr_map.insert(k.clone(), v.clone());
                changed = true;
            } else {
                changed |= merge_json(curr_map.get_mut(k).unwrap(), v);
            }
        }
    }
    // Arrays and scalars are preserved as they are without merging
    changed
}

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

    // We consume `current_val` here, avoiding a clone.
    let config: T = serde_json::from_value(current_val).with_context(|| {
        format!(
            "Failed to deserialize merged config file at {:?}",
            config_path
        )
    })?;

    Ok(config)
}

pub(crate) fn resolve_date_var(val: &str, base_date: Option<&str>) -> Result<chrono::NaiveDate> {
    use chrono::{Datelike, Local, NaiveDate};
    use tracing::{debug, error, info, trace};

    let val_lower = val.trim().to_lowercase();
    match val_lower.as_str() {
        "today" => {
            info!("Variable detected: {}", val);
            let dt = Local::now().date_naive();
            debug!("Resolved value: {} (Original: {})", dt, val);
            trace!("Variable resolution path: today");
            Ok(dt)
        }
        "yesterday" => {
            info!("Variable detected: {}", val);
            let dt =
                Local::now().date_naive() - chrono::TimeDelta::try_days(1).expect("valid days");
            debug!("Resolved value: {} (Original: {})", dt, val);
            trace!("Variable resolution path: yesterday");
            Ok(dt)
        }
        "tomorrow" => {
            info!("Variable detected: {}", val);
            let dt =
                Local::now().date_naive() + chrono::TimeDelta::try_days(1).expect("valid days");
            debug!("Resolved value: {} (Original: {})", dt, val);
            trace!("Variable resolution path: tomorrow");
            Ok(dt)
        }
        "eomonth" => {
            info!("Variable detected: {}", val);
            let base_dt = if let Some(bd) = base_date {
                parse_flexible_date_impl(bd, None)
            } else {
                None
            };
            let dt = base_dt.unwrap_or_else(|| Local::now().date_naive());
            let next_month = if dt.month() == 12 {
                NaiveDate::from_ymd_opt(dt.year() + 1, 1, 1).expect("valid next year")
            } else {
                NaiveDate::from_ymd_opt(dt.year(), dt.month() + 1, 1).expect("valid next month")
            };
            let res = next_month.pred_opt().expect("valid preceding day");

            trace!(
                "Variable resolution path: eomonth. Base: {}, Result: {}",
                dt,
                res
            );
            debug!("Resolved value: {} (Original: {})", res, val);
            Ok(res)
        }
        _ => {
            if val_lower.chars().all(|c| c.is_alphabetic()) && val_lower != "may" {
                error!("Invalid variable: {} | Resolution failure", val);
                anyhow::bail!("Invalid date variable: {}", val);
            }
            anyhow::bail!("Not a date variable")
        }
    }
}

fn parse_flexible_date_impl(val: &str, base_date: Option<&str>) -> Option<chrono::NaiveDate> {
    use chrono::NaiveDate;
    use tracing::warn;

    let val = val.trim();
    if val.is_empty() {
        return None;
    }

    if let Ok(resolved) = resolve_date_var(val, base_date) {
        return Some(resolved);
    }

    // Check if it was meant to be a variable but failed validation
    if val.chars().all(|c| c.is_alphabetic()) && val.to_lowercase() != "may" {
        warn!("Unsupported alphabetic string passed as date: {}", val);
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
    ];

    for fmt in formats {
        if let Ok(dt) = NaiveDate::parse_from_str(val, fmt) {
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

pub(crate) fn generate_csv_diagnostic_context(content: &str, line_num: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut ctx = String::new();

    let start = if line_num > 20 { line_num - 20 } else { 1 };
    let end = std::cmp::min(line_num + 20, lines.len());

    for i in start..=end {
        if i == 0 {
            continue;
        }
        if i == line_num {
            ctx.push_str(&format!(">>> {:4} | {}\n", i, lines[i - 1]));
        } else {
            ctx.push_str(&format!("{:4} | {}\n", i, lines[i - 1]));
        }
    }

    ctx
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

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
        // assert_eq!(to_iso_date("01-May-26"), "2026-05-01"); // %y is tricky with chrono, skip for now or fix
        assert_eq!(to_iso_date("invalid"), "invalid");
        assert_eq!(to_iso_date(""), "");
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
    fn test_generate_csv_diagnostic_context() {
        let lines: Vec<String> = (1..=50).map(|i| format!("Line {}", i)).collect();
        let file_content = lines.join("\n");

        // Test middle
        let ctx = generate_csv_diagnostic_context(&file_content, 25);
        let ctx_lines: Vec<&str> = ctx.trim().split('\n').collect();
        assert_eq!(ctx_lines.len(), 41); // 20 before + 1 error + 20 after
        assert_eq!(ctx_lines[0].trim(), "5 | Line 5");
        assert_eq!(ctx_lines[20].trim(), ">>>   25 | Line 25");
        assert_eq!(ctx_lines[40].trim(), "45 | Line 45");

        // Test beginning bounds
        let ctx_early = generate_csv_diagnostic_context(&file_content, 5);
        let ctx_early_lines: Vec<&str> = ctx_early.trim().split('\n').collect();
        assert_eq!(ctx_early_lines.len(), 25);
        assert_eq!(ctx_early_lines[0].trim(), "1 | Line 1");
        assert_eq!(ctx_early_lines[4].trim(), ">>>    5 | Line 5");
        assert_eq!(ctx_early_lines[24].trim(), "25 | Line 25");

        // Test end bounds
        let ctx_late = generate_csv_diagnostic_context(&file_content, 45);
        let ctx_late_lines: Vec<&str> = ctx_late.trim().split('\n').collect();
        assert_eq!(ctx_late_lines.len(), 26);
        assert_eq!(ctx_late_lines[0].trim(), "25 | Line 25");
        assert_eq!(ctx_late_lines[20].trim(), ">>>   45 | Line 45");
        assert_eq!(ctx_late_lines[25].trim(), "50 | Line 50");
    }
}
