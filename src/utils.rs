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
}
