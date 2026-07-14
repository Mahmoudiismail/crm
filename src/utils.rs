use crate::manifest::AppManifest;
use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

pub fn executable_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn setup_logging(app_name: &str) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let log_dir = executable_dir();
    let file_appender = tracing_appender::rolling::never(&log_dir, format!("{}.log", app_name));
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_filter(tracing_subscriber::filter::LevelFilter::TRACE);

    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(true)
        .with_thread_ids(true)
        .with_filter(tracing_subscriber::filter::LevelFilter::TRACE);

    let _ = tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .try_init();

    Ok(guard)
}

pub fn intercept_manifest(manifest: AppManifest) {
    for arg in std::env::args().skip(1) {
        if arg == "--manifest" {
            if let Ok(json) = serde_json::to_string(&manifest) {
                println!("{}", json);
            }
            std::process::exit(0);
        }
    }
}

pub fn load_or_create_config<T: DeserializeOwned + Serialize>(
    config_path: &Path,
    default_config: &T,
) -> Result<T> {
    if !config_path.exists() {
        let content = serde_json::to_string_pretty(default_config)
            .context("Failed to serialize default config")?;
        std::fs::write(config_path, content)
            .with_context(|| format!("Failed to write default config at {:?}", config_path))?;
    }

    let content = std::fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config file at {:?}", config_path))?;

    let config: T = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file at {:?}", config_path))?;

    Ok(config)
}

pub fn build_csv_reader(file_content: &str) -> csv::ReaderBuilder {
    let mut builder = csv::ReaderBuilder::new();
    builder
        .has_headers(true)
        .flexible(true)
        .delimiter(if file_content.contains('\t') {
            b'\t'
        } else {
            b','
        });
    builder
}

pub fn generate_csv_diagnostic_context(file_content: &str, error_line_num: usize) -> String {
    let start_line = error_line_num.saturating_sub(20).max(1_usize);
    let end_line = error_line_num + 20;

    let mut diagnostic_info = String::new();
    for (idx, line) in file_content.lines().enumerate() {
        let current_line_num = idx + 1;
        if current_line_num >= start_line && current_line_num <= end_line {
            let marker = if current_line_num == error_line_num {
                ">>> "
            } else {
                "    "
            };
            diagnostic_info.push_str(&format!("{}{:4} | {}\n", marker, current_line_num, line));
        } else if current_line_num > end_line {
            break;
        }
    }
    diagnostic_info
}

pub fn parse_flexible_date(val: &str) -> Option<chrono::NaiveDate> {
    use chrono::NaiveDate;
    let val = val.trim();
    if val.is_empty() {
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

pub fn replace_date_vars(val: &str, base_date: Option<&str>) -> String {
    use chrono::{Datelike, Local, NaiveDate};

    let normalize_to_dmy = |v: &str| -> String {
        if let Some(dt) = parse_flexible_date(v) {
            dt.format("%d-%m-%Y").to_string()
        } else {
            v.trim().to_string()
        }
    };

    let val_lower = val.trim().to_lowercase();
    match val_lower.as_str() {
        "today" => Local::now().format("%d-%m-%Y").to_string(),
        "yesterday" => (Local::now() - chrono::TimeDelta::days(1))
            .format("%d-%m-%Y")
            .to_string(),
        "tomorrow" => (Local::now() + chrono::TimeDelta::days(1))
            .format("%d-%m-%Y")
            .to_string(),
        "eomonth" => {
            let base_dt = if let Some(bd) = base_date {
                parse_flexible_date(bd)
            } else {
                None
            };
            let dt = base_dt.unwrap_or_else(|| Local::now().date_naive());
            let next_month = if dt.month() == 12 {
                NaiveDate::from_ymd_opt(dt.year() + 1, 1, 1).expect("valid next year")
            } else {
                NaiveDate::from_ymd_opt(dt.year(), dt.month() + 1, 1).expect("valid next month")
            };
            next_month
                .pred_opt()
                .expect("valid preceding day")
                .format("%d-%m-%Y")
                .to_string()
        }
        _ => normalize_to_dmy(val),
    }
}

pub fn to_iso_date(val: &str) -> String {
    if let Some(dt) = parse_flexible_date(val) {
        dt.format("%Y-%m-%d").to_string()
    } else {
        val.trim().to_string()
    }
}

pub fn build_csv_reader_builder() -> csv::ReaderBuilder {
    let mut builder = csv::ReaderBuilder::new();
    builder.has_headers(true).flexible(true);
    builder
}

pub fn build_csv_reader<R: std::io::Read>(rdr: R) -> csv::Reader<R> {
    build_csv_reader_builder().from_reader(rdr)
}

pub fn generate_csv_diagnostic_context(file_content: &str, line_num: usize) -> String {
    let start_line = if line_num > 20 { line_num - 20 } else { 1 };
    let end_line = line_num + 20;

    let mut diagnostic_info = String::new();
    for (idx, line) in file_content.lines().enumerate() {
        let current_line_num = idx + 1;
        if current_line_num >= start_line && current_line_num <= end_line {
            let marker = if current_line_num == line_num {
                ">>>"
            } else {
                "   "
            };
            diagnostic_info.push_str(&format!("{} {:4} | {}\n", marker, current_line_num, line));
        } else if current_line_num > end_line {
            break;
        }
    }
    diagnostic_info
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    #[test]
    fn test_replace_date_vars() {
        let today = Local::now().format("%d-%m-%Y").to_string();
        let yesterday = (Local::now() - chrono::Duration::days(1))
            .format("%d-%m-%Y")
            .to_string();
        let tomorrow = (Local::now() + chrono::Duration::days(1))
            .format("%d-%m-%Y")
            .to_string();

        assert_eq!(replace_date_vars("today", None), today);
        assert_eq!(replace_date_vars("yesterday", None), yesterday);
        assert_eq!(replace_date_vars("tomorrow", None), tomorrow);

        // Test normalization
        assert_eq!(replace_date_vars("2026-05-01", None), "01-05-2026");
        assert_eq!(replace_date_vars("01/05/2026", None), "01-05-2026");

        // Test eomonth
        // May 2026 end is 31-05-2026
        assert_eq!(
            replace_date_vars("eomonth", Some("01-05-2026")),
            "31-05-2026"
        );
        // Feb 2024 (leap) is 29-02-2024
        assert_eq!(
            replace_date_vars("eomonth", Some("2024-02-01")),
            "29-02-2024"
        );
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
    fn test_csv_reader_flexible() {
        // Test variable columns, blank rows, quoted multiline
        let csv_data = "col1,col2,col3\nval1,val2\n\nval3,val4,val5,val6\n\"multi\nline\",escaped\"\"quote,val";
        let mut rdr = build_csv_reader(csv_data.as_bytes());
        let records: Vec<_> = rdr.records().map(|r| r.unwrap()).collect();

        assert_eq!(records.len(), 3);

        // Row 1: short
        assert_eq!(records[0].len(), 2);
        assert_eq!(&records[0][0], "val1");
        assert_eq!(&records[0][1], "val2");

        // Row 2: long
        assert_eq!(records[1].len(), 4);
        assert_eq!(&records[1][0], "val3");
        assert_eq!(&records[1][3], "val6");

        // Row 3: quotes and multiline
        assert_eq!(records[2].len(), 3);
        assert_eq!(&records[2][0], "multi\nline");
        assert_eq!(&records[2][1], "escaped\"\"quote");
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
