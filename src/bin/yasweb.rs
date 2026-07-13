use anyhow::{Context, Result};
use chrono::{Datelike, Local, NaiveDate};
use crm_tool::manifest::{AppArg, AppManifest, ArgType};

use headless_chrome::{Browser, LaunchOptions};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use crm_tool::utils::{executable_dir, intercept_manifest, setup_logging};
use crm_tool::yasweb::browser::run_browser_tab;
use crm_tool::yasweb::config::{ReportConfig, YaswebConfig};
use tokio::fs;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "yasweb", about = "YAS Web Scraper")]
pub struct YaswebCliOptions {
    #[arg(long)]
    name: String,
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

                    let start_key = report.start_date_key.clone().unwrap_or_default();
                    let end_key = report.end_date_key.clone().unwrap_or_default();

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

    let mut arguments = vec![
        AppArg {
            name: "--config".to_string(),
            arg_type: ArgType::String,
            required: false,
            default_value: None,
            options: None,
            depends_on: None,
            autofill: None,
        },
        AppArg {
            name: "--name".to_string(),
            arg_type: if report_names.is_empty() {
                ArgType::String
            } else {
                ArgType::List
            },
            required: true,
            default_value: None,
            options: if report_names.is_empty() {
                None
            } else {
                Some(report_names)
            },
            depends_on: None,
            autofill: None,
        },
        AppArg {
            name: "--type".to_string(),
            arg_type: ArgType::List,
            required: false,
            default_value: None,
            options: Some(vec![
                "".to_string(),
                "Standard Report".to_string(),
                "Report Manager".to_string(),
            ]),
            depends_on: None,
            autofill: if type_autofills.is_empty() {
                None
            } else {
                let mut map = std::collections::HashMap::new();
                map.insert("--name".to_string(), type_autofills);
                Some(map)
            },
        },
        AppArg {
            name: "--monthly".to_string(),
            arg_type: ArgType::Boolean,
            required: false,
            default_value: None,
            options: None,
            depends_on: None,
            autofill: None,
        },
        AppArg {
            name: "--start-date".to_string(),
            arg_type: ArgType::String,
            required: false,
            default_value: None,
            options: None,
            depends_on: None,
            autofill: None,
        },
        AppArg {
            name: "--end-date".to_string(),
            arg_type: ArgType::String,
            required: false,
            default_value: None,
            options: None,
            depends_on: None,
            autofill: None,
        },
        AppArg {
            name: "--add-time-to-file".to_string(),
            arg_type: ArgType::Boolean,
            required: false,
            default_value: None,
            options: None,
            depends_on: None,
            autofill: None,
        },
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

        arguments.push(AppArg {
            name: format!("--filter-{}", f_name),
            arg_type: ArgType::String,
            required: false,
            default_value: None,
            options: None,
            depends_on: Some(depends_map),
            autofill: autofill_opt,
        });
    }

    AppManifest {
        name: "Yasweb Automation Engine".to_string(),
        description: "Executes headless browser automation for web reporting.".to_string(),
        arguments,
    }
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

    let _guard = setup_logging("yasweb")?;
    let options = YaswebCliOptions::parse();

    let p = PathBuf::from(options.config);
    let config_path = if p.is_absolute() {
        p
    } else {
        executable_dir().join(p)
    };

    let mut config: YaswebConfig = crm_tool::utils::load_or_create_config(
        &config_path,
        &YaswebConfig {
            url: "https://example.com".to_string(),
            username: "user".to_string(),
            password: Some("pass".to_string()),
            reports: HashMap::new(),
            headless: false,
            keep_open: false,
            concurrency: 6,
        },
    )?;
    let mut config_updated = false;

    let active_report_name = options.name;
    let mut active_report_type = options.r#type.unwrap_or_default();

    let mut active_filters: HashMap<String, String> = HashMap::new();
    if let Some(filters_str) = options.filters {
        if !filters_str.trim().is_empty() {
            active_filters =
                serde_json::from_str(&filters_str).context("Failed to parse filters JSON")?;
        }
    }

    let is_monthly = options.monthly;
    let mut start_date_str = options.start_date;
    let mut end_date_str = options.end_date;
    let add_time_to_file = options.add_time_to_file;

    if active_report_name.is_empty() {
        error!("Validation failed: --name is required.");
        anyhow::bail!("Validation failed: --name is required.");
    }

    use crm_tool::utils::replace_date_vars;

    if let Some(ref s) = start_date_str {
        start_date_str = Some(replace_date_vars(s, None));
    }

    // For eomonth in end_date, pass the start_date as base
    if let Some(ref e) = end_date_str {
        let base = start_date_str.as_deref();
        end_date_str = Some(replace_date_vars(e, base));
    }

    let mut new_filters = HashMap::new();
    for (k, v) in active_filters.into_iter() {
        let base = start_date_str.as_deref();
        new_filters.insert(k, replace_date_vars(&v, base));
    }
    active_filters = new_filters;

    // Determine configuration to use
    if !active_report_type.is_empty() || !active_filters.is_empty() {
        // We received details from CLI.
        // Try to preserve existing date keys if they exist in the current config
        let (existing_start_key, existing_end_key) =
            if let Some(existing) = config.reports.get(&active_report_name) {
                (
                    existing.start_date_key.clone(),
                    existing.end_date_key.clone(),
                )
            } else {
                (None, None)
            };

        let report_conf = ReportConfig {
            report_type: active_report_type.clone(),
            filters: active_filters.clone(),
            start_date_key: existing_start_key,
            end_date_key: existing_end_key,
        };
        config
            .reports
            .insert(active_report_name.clone(), report_conf);
        config_updated = true;
    } else {
        // Retrieve from config
        if let Some(cached) = config.reports.get(&active_report_name) {
            info!("Found cached configuration for '{}'", active_report_name);
            active_report_type = cached.report_type.clone();
            active_filters = cached.filters.clone();
        } else {
            error!(
                "Report '{}' not found in config and no --type/--filters provided via CLI.",
                active_report_name
            );
            anyhow::bail!(
                "Report '{}' not found in config and no --type/--filters provided via CLI.",
                active_report_name
            );
        }
    }

    if config_updated {
        info!("Updating yasweb_config.json with CLI report parameters...");
        let content = serde_json::to_string_pretty(&config)
            .context("Failed to serialize updated yasweb config")?;
        fs::write(&config_path, content)
            .await
            .context("Failed to write updated yasweb_config.json")?;
        info!("yasweb_config.json updated successfully.");
    }

    info!("Loaded config for URL: {}", config.url);

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
                .is_none_or(|k| k.is_empty())
            || report_conf
                .end_date_key
                .as_ref()
                .is_none_or(|k| k.is_empty())
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
            anyhow::bail!("--start-date must be before or equal to --end-date");
        }

        let mut current_dt = start_dt;
        while current_dt <= end_dt {
            let next_month = if current_dt.month() == 12 {
                NaiveDate::from_ymd_opt(current_dt.year() + 1, 1, 1).context("Invalid date math")?
            } else {
                NaiveDate::from_ymd_opt(current_dt.year(), current_dt.month() + 1, 1)
                    .context("Invalid date math")?
            };

            let last_day = next_month.pred_opt().context("Invalid date math")?;
            let chunk_end = if last_day > end_dt { end_dt } else { last_day };

            date_ranges.push((
                current_dt.format("%d-%m-%Y").to_string(),
                chunk_end.format("%d-%m-%Y").to_string(),
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

    let config_path_clone = config_path.clone();
    let mut config_clone = config.clone();
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
                start_date_str.clone()
            };
            let e_dt = if is_monthly && !end_dt.is_empty() {
                Some(end_dt.clone())
            } else {
                end_date_str.clone()
            };

            if let Some(report_conf) = report_conf_opt {
                if let Some(sk) = &report_conf.start_date_key {
                    if let Some(s) = s_dt.clone() {
                        run_filters.insert(sk.clone(), s);
                    }
                }
                if let Some(ek) = &report_conf.end_date_key {
                    if let Some(e) = e_dt.clone() {
                        run_filters.insert(ek.clone(), e);
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
            // Move and rename the file
            if let Ok(entries) = std::fs::read_dir(&temp_dl_dir) {
                let mut final_out_dir = executable_dir();
                final_out_dir.push("downloads");
                let _ = std::fs::create_dir_all(&final_out_dir);

                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "xlsx" || ext == "xls" || ext == "csv" {
                            let mut out_file = final_out_dir.clone();
                            // If the original file was not xlsx, adapt the extension, but we requested XLSX
                            let ext_str = ext.to_string_lossy().to_string();
                            let mut final_name = final_filename.clone();
                            if ext_str != "xlsx" {
                                final_name = final_name.replace(".xlsx", &format!(".{}", ext_str));
                            }
                            out_file.push(final_name);

                            info!("Moving downloaded file from {:?} to {:?}", path, out_file);
                            if let Err(e) = std::fs::rename(&path, &out_file) {
                                error!("Failed to rename/move file: {}", e);
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
            let _ = std::fs::remove_dir_all(&temp_dl_dir);

            if !discovered_filters.is_empty() {
                if let Some(report) = config_clone.reports.get_mut(&active_report_name_clone) {
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
                        info!("Updating yasweb_config.json with newly discovered filters...");
                        let content = serde_json::to_string_pretty(&config_clone)
                            .context("Failed to serialize updated yasweb config")?;
                        // Best effort write
                        let _ = fs::write(&config_path_clone, content).await;
                    }
                }
            }
        }
    }

    Ok(())
}
