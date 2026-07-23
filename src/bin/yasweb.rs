use anyhow::{Context, Result};
use chrono::{Datelike, Local, NaiveDate};
use crm_tool::manifest::{AppArg, AppManifest, ArgType};

use headless_chrome::{Browser, LaunchOptions};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use crm_tool::utils::{
    executable_dir, intercept_manifest, parse_log_level, setup_logging_with_levels,
};
use crm_tool::yasweb::browser::run_browser_tab;
use crm_tool::yasweb::config::{ReportConfig, YaswebConfig};
use tokio::fs;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "yasweb", about = "YAS Web Scraper")]
pub struct YaswebCliOptions {
    #[arg(long, value_delimiter = ',')]
    name: Vec<String>,
    #[arg(long)]
    r#type: Option<String>,
    #[arg(long, default_value = "yasweb_config.json")]
    config: String,
    #[arg(long)]
    filters: Option<String>,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    monthly: bool,
    #[arg(long)]
    start_date: Option<String>,
    #[arg(long)]
    end_date: Option<String>,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    add_time_to_file: bool,
    #[arg(long, hide = true)]
    manifest: bool,
}

fn get_manifest(config_path: Option<PathBuf>) -> AppManifest {
    let mut report_names = Vec::new();
    let mut filter_dependencies: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut type_autofills: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut filter_autofills: std::collections::HashMap<
        String,
        std::collections::HashMap<String, String>,
    > = std::collections::HashMap::new();

    if let Some(path) = config_path.clone() {
        if let Ok(config_str) = std::fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<YaswebConfig>(&config_str) {
                for (name, report) in &config.reports {
                    report_names.push(name.clone());

                    if !report.report_type.trim().is_empty() {
                        type_autofills.insert(name.clone(), report.report_type.clone());
                    }

                    let start_key = report
                        .start_date_key
                        .clone()
                        .map(|k| k.key)
                        .unwrap_or_default();
                    let end_key = report
                        .end_date_key
                        .clone()
                        .map(|k| k.key)
                        .unwrap_or_default();

                    for (filter_key, filter_val) in &report.filters {
                        // Skip generating filter args for standard date keys since we map --start-date/--end-date to them
                        if filter_key == &start_key || filter_key == &end_key {
                            continue;
                        }
                        filter_dependencies
                            .entry(filter_key.clone())
                            .or_default()
                            .push(name.clone());

                        if !filter_val.trim().is_empty() {
                            filter_autofills
                                .entry(filter_key.clone())
                                .or_default()
                                .insert(name.clone(), filter_val.clone());
                        }
                    }
                }
            }
        }
    }

    let arg_name = AppArg::new(
        "--name",
        if report_names.is_empty() {
            ArgType::String
        } else {
            ArgType::MultiList
        },
    )
    .required(true);
    let arg_name = if report_names.is_empty() {
        arg_name
    } else {
        arg_name.options(report_names)
    };

    let mut type_map = std::collections::HashMap::new();
    if !type_autofills.is_empty() {
        type_map.insert("--name".to_string(), type_autofills);
    }

    let mut arg_type = AppArg::new("--type", ArgType::List).options(vec![
        "".to_string(),
        "Standard Report".to_string(),
        "Report Manager".to_string(),
    ]);
    if !type_map.is_empty() {
        arg_type = arg_type.autofill(type_map);
    }

    let mut arguments = vec![
        AppArg::new("--config", ArgType::String),
        arg_name,
        arg_type,
        AppArg::new("--monthly", ArgType::Boolean),
        AppArg::new("--start-date", ArgType::DateVar),
        AppArg::new("--end-date", ArgType::DateVar),
        AppArg::new("--add-time-to-file", ArgType::Boolean),
    ];

    let mut sorted_filters: Vec<_> = filter_dependencies.into_iter().collect();
    sorted_filters.sort_by(|a, b| a.0.cmp(&b.0));
    for (f_name, deps) in sorted_filters {
        let mut depends_map = std::collections::HashMap::new();
        depends_map.insert("--name".to_string(), deps);

        let autofill_opt = if let Some(fills) = filter_autofills.get(&f_name) {
            let mut map = std::collections::HashMap::new();
            map.insert("--name".to_string(), fills.clone());
            Some(map)
        } else {
            None
        };

        let mut arg =
            AppArg::new(format!("--filter-{}", f_name), ArgType::String).depends_on(depends_map);
        if let Some(auto) = autofill_opt {
            arg = arg.autofill(auto);
        }
        arguments.push(arg);
    }

    AppManifest {
        name: "Yasweb Automation Engine".to_string(),
        description: "Executes headless browser automation for web reporting.".to_string(),
        arguments,
    }
}

pub fn finalize_download(
    temp_dl_dir: &std::path::Path,
    final_out_dir: &std::path::Path,
    final_filename: &str,
) -> anyhow::Result<()> {
    // Move and rename the file
    if let Ok(entries) = std::fs::read_dir(temp_dl_dir) {
        let _ = std::fs::create_dir_all(final_out_dir);

        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "xlsx" || ext == "xls" || ext == "csv" {
                    let mut out_file = final_out_dir.to_path_buf();
                    // If the original file was not xlsx, adapt the extension, but we requested XLSX
                    let ext_str = ext.to_string_lossy().to_string();
                    let mut final_name = final_filename.to_string();
                    if ext_str != "xlsx" {
                        final_name = final_name.replace(".xlsx", &format!(".{}", ext_str));
                    }
                    out_file.push(final_name);

                    tracing::info!("Moving downloaded file from {:?} to {:?}", path, out_file);
                    if let Err(e) = std::fs::rename(&path, &out_file) {
                        tracing::error!("Failed to rename/move file: {}", e);
                        // Fallback to copy+delete across mount points
                        if std::fs::copy(&path, &out_file).is_ok() {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    }

    // Cleanup temp dir
    let _ = std::fs::remove_dir_all(temp_dl_dir);
    Ok(())
}
#[tokio::main]
async fn main() -> Result<()> {
    // Pre-parse args to look for --manifest and try to parse --config manually first
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--manifest") {
        let mut config_path_opt = None;
        for i in 0..args.len() {
            if args[i] == "--config" && i + 1 < args.len() {
                config_path_opt = Some(PathBuf::from(&args[i + 1]));
                break;
            }
        }

        let config_path =
            config_path_opt.unwrap_or_else(|| executable_dir().join("yasweb_config.json"));
        intercept_manifest(get_manifest(Some(config_path)));
    }

    let options = YaswebCliOptions::parse();

    let p = PathBuf::from(options.config);
    let config_path = if p.is_absolute() {
        p
    } else {
        executable_dir().join(p)
    };

    let mut config: YaswebConfig =
        crm_tool::utils::load_or_create_config(&config_path, &YaswebConfig::default())?;

    let _guard = setup_logging_with_levels(
        "yasweb",
        parse_log_level(&config.log_stdout_level),
        parse_log_level(&config.log_file_level),
    )?;
    let mut config_updated = false;

    // Config healing for empty formatting
    for report in config.reports.values_mut() {
        let (default_start_fmt, default_end_fmt) = if report.report_type == "Report Manager" {
            ("%d-%b-%Y".to_string(), "%d-%b-%Y".to_string())
        } else {
            ("%d-%m-%Y 00:00".to_string(), "%d-%m-%Y 23:59".to_string())
        };

        if let Some(ref mut sk) = report.start_date_key {
            if sk.format.is_empty() {
                sk.format = default_start_fmt.clone();
                config_updated = true;
            }
        }
        if let Some(ref mut ek) = report.end_date_key {
            if ek.format.is_empty() {
                ek.format = default_end_fmt.clone();
                config_updated = true;
            }
        }
    }

    let active_report_names = options.name;
    let active_report_type_global = options.r#type.unwrap_or_default();

    let mut active_filters_global: HashMap<String, String> = HashMap::new();
    if let Some(filters_str) = options.filters {
        if !filters_str.trim().is_empty() {
            active_filters_global =
                serde_json::from_str(&filters_str).context("Failed to parse filters JSON")?;
        }
    }

    let is_monthly = options.monthly;
    let start_date_str_global = options.start_date;
    let end_date_str_global = options.end_date;
    let add_time_to_file = options.add_time_to_file;

    if active_report_names.is_empty() {
        error!("Validation failed: --name is required.");
        anyhow::bail!("Validation failed: --name is required.");
    }

    use crm_tool::utils::replace_date_vars;

    let start_date_str_global = start_date_str_global.map(|s| replace_date_vars(&s, None));
    let end_date_str_global =
        end_date_str_global.map(|e| replace_date_vars(&e, start_date_str_global.as_deref()));

    let mut new_filters_global = HashMap::new();
    for (k, v) in active_filters_global.into_iter() {
        let base = start_date_str_global.as_deref();
        new_filters_global.insert(k, replace_date_vars(&v, base));
    }
    active_filters_global = new_filters_global;

    let _config_path_clone = config_path.clone();

    // 1. First, sequentially update config for all reports if CLI params were provided
    let mut config_updated_global = config_updated;
    for active_report_name in &active_report_names {
        if !active_report_type_global.is_empty() || !active_filters_global.is_empty() {
            let (existing_start_key, existing_end_key) =
                if let Some(existing) = config.reports.get(active_report_name) {
                    (
                        existing.start_date_key.clone(),
                        existing.end_date_key.clone(),
                    )
                } else {
                    (None, None)
                };

            let report_conf = ReportConfig {
                report_type: active_report_type_global.clone(),
                filters: active_filters_global.clone(),
                start_date_key: existing_start_key,
                end_date_key: existing_end_key,
            };
            config
                .reports
                .insert(active_report_name.clone(), report_conf);
            config_updated_global = true;
        }
    }

    if config_updated_global {
        info!("Updating yasweb_config.json with CLI report parameters...");
        let content = serde_json::to_string_pretty(&config)
            .context("Failed to serialize updated yasweb config")?;
        fs::write(&config_path, content)
            .await
            .context("Failed to write updated yasweb_config.json")?;
        info!("yasweb_config.json updated successfully.");
    }

    info!("Loaded config for URL: {}", config.url);

    let mut report_futures = Vec::new();

    // Iterate over active_report_names
    for active_report_name in active_report_names {
        let active_report_type = active_report_type_global.clone();
        let active_filters = active_filters_global.clone();
        let start_date_str = start_date_str_global.clone();
        let end_date_str = end_date_str_global.clone();

        let config = config.clone();
        let config_path_clone = config_path.clone();

        let active_report_name_clone_for_future = active_report_name.clone();

        // Push an async block into our futures list
        report_futures.push(async move {
            let active_report_name = active_report_name_clone_for_future;
            let mut active_report_type = active_report_type;
            let mut active_filters = active_filters;

            // Retrieve from config
            if let Some(cached) = config.reports.get(&active_report_name) {
                info!("Found cached configuration for '{}'", active_report_name);
                if active_report_type.is_empty() {
                    active_report_type = cached.report_type.clone();
                }
                if active_filters.is_empty() {
                    active_filters = cached.filters.clone();
                }
            } else {
                error!(
                    "Report '{}' not found in config and no --type/--filters provided via CLI.",
                    active_report_name
                );
                return Err(anyhow::anyhow!(
                    "Report '{}' not found in config and no --type/--filters provided via CLI.",
                    active_report_name
                ));
            }

            // Parse date ranges
            let mut date_ranges: Vec<(String, String)> = Vec::new();

            let mut effective_monthly = is_monthly;
            if effective_monthly {
                let report_conf = config
                    .reports
                    .get(&active_report_name)
                    .context("Report name not found in config")?;

                if report_conf.start_date_key.is_none()
                    || report_conf.end_date_key.is_none()
                    || report_conf
                        .start_date_key
                        .as_ref()
                        .is_none_or(|k| k.key.is_empty())
                    || report_conf
                        .end_date_key
                        .as_ref()
                        .is_none_or(|k| k.key.is_empty())
                {
                    info!("Report does not have start_date_key/end_date_key configured. Disabling monthly chunking.");
                    effective_monthly = false;
                }
            }

            if effective_monthly {
                let start_date = start_date_str
                    .clone()
                    .context("--start-date is required when --monthly is true")?;
                let end_date = end_date_str
                    .clone()
                    .context("--end-date is required when --monthly is true")?;

                use crm_tool::utils::parse_flexible_date;
                let start_dt = parse_flexible_date(&start_date).context("Invalid --start-date format")?;
                let end_dt = parse_flexible_date(&end_date).context("Invalid --end-date format")?;

                if start_dt > end_dt {
                    return Err(anyhow::anyhow!("--start-date must be before or equal to --end-date"));
                }

                let mut current_dt = start_dt;

                let report_conf_monthly = config
                    .reports
                    .get(&active_report_name)
                    .context("Report name not found in config")?;

                while current_dt <= end_dt {
                    let next_month = if current_dt.month() == 12 {
                        NaiveDate::from_ymd_opt(current_dt.year() + 1, 1, 1).context("Invalid date math")?
                    } else {
                        NaiveDate::from_ymd_opt(current_dt.year(), current_dt.month() + 1, 1)
                            .context("Invalid date math")?
                    };

                    let last_day = next_month.pred_opt().context("Invalid date math")?;
                    let chunk_end = if last_day > end_dt { end_dt } else { last_day };

                    let start_format_str = if let Some(ref sk) = report_conf_monthly.start_date_key {
                        sk.format.clone()
                    } else if active_report_type == "Report Manager" {
                        "%d-%b-%Y".to_string()
                    } else {
                        "%d-%m-%Y 00:00".to_string()
                    };

                    let end_format_str = if let Some(ref ek) = report_conf_monthly.end_date_key {
                        ek.format.clone()
                    } else if active_report_type == "Report Manager" {
                        "%d-%b-%Y".to_string()
                    } else {
                        "%d-%m-%Y 23:59".to_string()
                    };

                    let start_dt_time = current_dt.and_hms_opt(0, 0, 0).unwrap();
                    let end_dt_time = chunk_end.and_hms_opt(23, 59, 59).unwrap();

                    date_ranges.push((
                        start_dt_time.format(&start_format_str).to_string(),
                        end_dt_time.format(&end_format_str).to_string(),
                    ));

                    current_dt = next_month;
                }

                info!(
                    "Monthly execution planned for {} ranges: {:?}",
                    date_ranges.len(),
                    date_ranges
                );
            } else {
                // Not monthly, just add a dummy single entry
                date_ranges.push(("".to_string(), "".to_string()));
            }

            let config_clone = config.clone();
            let active_report_name_clone = active_report_name.clone();

            // Launch browser once
            let mut user_data_dir = executable_dir();
            user_data_dir.push("yasweb_chrome_data");
            // Use unique data dir per report to allow simultaneous runs
            let safe_name = active_report_name.replace(|c: char| !c.is_alphanumeric(), "_");
            user_data_dir.push(safe_name);

            let args = vec![
                std::ffi::OsStr::new("--ignore-certificate-errors"),
                std::ffi::OsStr::new("--start-maximized"),
                std::ffi::OsStr::new("--disable-web-security"),
                std::ffi::OsStr::new("--disable-site-isolation-trials"),
                std::ffi::OsStr::new("--disable-features=IsolateOrigins,site-per-process"),
                std::ffi::OsStr::new("--disable-session-crashed-bubble"),
                std::ffi::OsStr::new("--no-first-run"),
                std::ffi::OsStr::new("--disable-infobars"),
                std::ffi::OsStr::new("--skip-reopen-last-pages"),
            ];

            let launch_options = LaunchOptions::default_builder()
                .headless(config.headless)
                .sandbox(false)
                .idle_browser_timeout(std::time::Duration::from_secs(120))
                .user_data_dir(Some(user_data_dir))
                .args(args)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build launch options: {e}"))?;

            let browser = Arc::new(Browser::new(launch_options).context("Failed to launch browser")?);

            // Chunk runs into concurrent batches based on config
            let concurrency = if config.concurrency == 0 {
                1
            } else {
                config.concurrency
            };

            for chunk in date_ranges.chunks(concurrency) {
                let mut tasks = Vec::new();
                for (i, (start_dt, end_dt)) in chunk.iter().enumerate() {
                    let config_task = config.clone();
                    let active_report_name_task = active_report_name.clone();
                    let active_report_type_task = active_report_type.clone();

                    let mut run_filters = active_filters.clone();
                    let report_conf_opt = config.reports.get(&active_report_name);

                    // Map the global --start-date and --end-date values into the respective internal report filters
                    // This is done both for monthly chunking AND standard executions, provided the report has the keys configured.
                    let s_dt = if is_monthly && !start_dt.is_empty() {
                        Some(start_dt.clone())
                    } else {
                        // If not monthly, re-format the input start_date_str if applicable
                        if let (Some(st_str), Some(report_conf)) = (&start_date_str, report_conf_opt) {
                            if let Some(sk) = &report_conf.start_date_key {
                                if let Some(parsed) = crm_tool::utils::parse_flexible_date(st_str) {
                                    let parsed_dt = parsed.and_hms_opt(0, 0, 0).unwrap();
                                    Some(parsed_dt.format(&sk.format).to_string())
                                } else {
                                    start_date_str.clone()
                                }
                            } else {
                                start_date_str.clone()
                            }
                        } else {
                            start_date_str.clone()
                        }
                    };

                    let e_dt = if is_monthly && !end_dt.is_empty() {
                        Some(end_dt.clone())
                    } else {
                        // If not monthly, re-format the input end_date_str if applicable
                        if let (Some(ed_str), Some(report_conf)) = (&end_date_str, report_conf_opt) {
                            if let Some(ek) = &report_conf.end_date_key {
                                if let Some(parsed) = crm_tool::utils::parse_flexible_date(ed_str) {
                                    let parsed_dt = parsed.and_hms_opt(23, 59, 59).unwrap();
                                    Some(parsed_dt.format(&ek.format).to_string())
                                } else {
                                    end_date_str.clone()
                                }
                            } else {
                                end_date_str.clone()
                            }
                        } else {
                            end_date_str.clone()
                        }
                    };

                    if let Some(report_conf) = report_conf_opt {
                        if let Some(sk) = &report_conf.start_date_key {
                            if let Some(s) = s_dt.clone() {
                                run_filters.insert(sk.key.clone(), s);
                            }
                        }
                        if let Some(ek) = &report_conf.end_date_key {
                            if let Some(e) = e_dt.clone() {
                                run_filters.insert(ek.key.clone(), e);
                            }
                        }
                    }

                    let browser_clone = browser.clone();
                    let is_initial = i == 0 && !chunk.is_empty(); // use initial tab for the first element in the batch to avoid lingering blank tabs

                    // Setup download dir for this tab
                    let date_suffix = if is_monthly && !start_dt.is_empty() {
                        // e.g. "01-01-2026", get MM-YYYY
                        let parts: Vec<&str> = start_dt.split('-').collect();
                        if parts.len() == 3 {
                            format!("{}-{}", parts[1], parts[2])
                        } else {
                            start_dt.clone()
                        }
                    } else if let Some(st) = start_date_str.clone() {
                        st
                    } else {
                        chrono::Local::now().format("%d-%m-%Y").to_string()
                    };

                    let temp_dl_dir = {
                        let mut dir = executable_dir();
                        dir.push(format!(
                            "yasweb_downloads_tmp_{}_{}_{}",
                            active_report_name, date_suffix, i
                        ));
                        let _ = std::fs::create_dir_all(&dir);
                        dir
                    };

                    let temp_dl_dir_clone = temp_dl_dir.clone();

                    let mut final_filename = format!("{}_{}.xlsx", active_report_name, date_suffix);
                    if add_time_to_file {
                        let time_suffix = Local::now().format("%H%M%S").to_string();
                        final_filename = format!(
                            "{}_{}_{}.xlsx",
                            active_report_name, date_suffix, time_suffix
                        );
                    }

                    tracing::trace!("Spawning task for range: {} to {}", start_dt, end_dt);
                    tasks.push(tokio::task::spawn_blocking(move || {
                        tracing::info!("Starting browser tab automation for task {}...", i);
                        let res = match run_browser_tab(
                            browser_clone,
                            &config_task,
                            &active_report_name_task,
                            &active_report_type_task,
                            &run_filters,
                            is_initial,
                            Some(temp_dl_dir.clone()),
                        ) {
                            Ok(filters) => {
                                tracing::info!(
                                    "Browser tab automation for task {} completed successfully.",
                                    i
                                );
                                filters
                            }
                            Err(e) => {
                                error!("Browser automation failed for task {}: {:?}", i, e);
                                eprintln!("Browser automation failed for task {}: {:?}", i, e);
                                Vec::new()
                            }
                        };
                        (res, temp_dl_dir_clone, final_filename)
                    }));
                }

                let results = futures_util::future::join_all(tasks).await;

                for (discovered_filters, temp_dl_dir, final_filename) in results.into_iter().flatten() {
                    let mut final_out_dir = executable_dir();
                    final_out_dir.push("downloads");
                    let _ = finalize_download(&temp_dl_dir, &final_out_dir, &final_filename);

                    if !discovered_filters.is_empty() {
                        // Reload config from disk to avoid race conditions with other concurrent reports
                        let latest_config_content = fs::read_to_string(&config_path_clone).await.unwrap_or_default();
                        let mut latest_config: YaswebConfig = match serde_json::from_str(&latest_config_content) {
                            Ok(c) => c,
                            Err(_) => config_clone.clone(), // Fallback to our initial config state
                        };

                        if let Some(report) = latest_config.reports.get_mut(&active_report_name_clone) {
                            let mut updated_filters = false;
                            for f in discovered_filters {
                                if let std::collections::hash_map::Entry::Vacant(e) =
                                    report.filters.entry(f)
                                {
                                    e.insert("".to_string());
                                    updated_filters = true;
                                }
                            }
                            if updated_filters {
                                info!("Updating yasweb_config.json with newly discovered filters for {}...", active_report_name_clone);
                                let content = match serde_json::to_string_pretty(&latest_config) {
                                    Ok(c) => c,
                                    Err(_) => continue,
                                };
                                // Best effort write
                                let _ = fs::write(&config_path_clone, content).await;
                            }
                        }
                    }
                }
            }
            Ok::<(), anyhow::Error>(())
        });
    }

    let results = futures_util::future::join_all(report_futures).await;
    for res in results {
        if let Err(e) = res {
            error!("Report execution failed: {:?}", e);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests_yasweb_finalization {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_finalize_download_rename_success() {
        let tmp = std::env::temp_dir();
        let src_dir = tmp.join("yasweb_test_src_1");
        let dst_dir = tmp.join("yasweb_test_dst_1");
        std::fs::create_dir_all(&src_dir).unwrap();

        let src_file = src_dir.join("temp.xlsx");
        let mut f = File::create(&src_file).unwrap();
        f.write_all(b"test data").unwrap();

        finalize_download(&src_dir, &dst_dir, "final_report.xlsx").unwrap();

        assert!(dst_dir.join("final_report.xlsx").exists());
        assert!(!src_dir.exists()); // src dir is removed
        std::fs::remove_dir_all(&dst_dir).unwrap();
    }

    #[test]
    fn test_finalize_download_missing_source() {
        let tmp = std::env::temp_dir();
        let src_dir = tmp.join("yasweb_test_src_missing");
        let dst_dir = tmp.join("yasweb_test_dst_missing");
        // We do not create src_dir to simulate missing source

        // Should not error, just does nothing cleanly
        finalize_download(&src_dir, &dst_dir, "final_report.xlsx").unwrap();
        assert!(!dst_dir.join("final_report.xlsx").exists());
    }

    #[test]
    fn test_finalize_download_unicode_spaces() {
        let tmp = std::env::temp_dir();
        let src_dir = tmp.join("yasweb_test_src_uni");
        let dst_dir = tmp.join("yasweb test 🚀 dst");
        std::fs::create_dir_all(&src_dir).unwrap();

        let src_file = src_dir.join("data.csv"); // tests extension swap
        let mut f = File::create(&src_file).unwrap();
        f.write_all(b"test").unwrap();

        finalize_download(&src_dir, &dst_dir, "report 🚀 data.xlsx").unwrap();

        // because the source is csv, it replaces .xlsx with .csv in the final name
        assert!(dst_dir.join("report 🚀 data.csv").exists());
        std::fs::remove_dir_all(&dst_dir).unwrap();
    }

    #[test]
    fn test_finalize_download_overwrite_existing() {
        let tmp = std::env::temp_dir();
        let src_dir = tmp.join("yasweb_test_src_ow");
        let dst_dir = tmp.join("yasweb_test_dst_ow");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();

        let src_file = src_dir.join("temp.xlsx");
        File::create(&src_file)
            .unwrap()
            .write_all(b"new data")
            .unwrap();

        let dst_file = dst_dir.join("final.xlsx");
        File::create(&dst_file)
            .unwrap()
            .write_all(b"old data")
            .unwrap();

        finalize_download(&src_dir, &dst_dir, "final.xlsx").unwrap();

        let content = std::fs::read_to_string(dst_file).unwrap();
        // Since rename on windows doesn't overwrite, but the fallback copy+delete will overwrite
        assert_eq!(content, "new data");
        std::fs::remove_dir_all(&dst_dir).unwrap();
    }
}
