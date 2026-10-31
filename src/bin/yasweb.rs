use anyhow::{Context, Result};
use chrono::Local;
use crm_tool::manifest::{AppArg, AppManifest, ArgType};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::Parser;
use crm_tool::utils::{
    executable_dir, intercept_manifest, parse_log_level, setup_logging_with_levels, InterceptResult,
};
use crm_tool::yasweb::browser::run_browser_tab;
use crm_tool::yasweb::config::{DiscoveredReportFilters, ReportConfig, YaswebConfig};
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
    #[arg(long)]
    start_date: Option<String>,
    #[arg(long)]
    end_date: Option<String>,
    #[arg(long, hide = true, action = clap::ArgAction::SetTrue)]
    monthly: bool,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    add_time_to_file: bool,
    #[arg(long)]
    download_path: Option<String>,
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
    let mut filter_options: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

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
                        if let Some(options) = report.filter_options.get(filter_key) {
                            let cached = filter_options.entry(filter_key.clone()).or_default();
                            for option in options {
                                if !option.trim().is_empty() && !cached.contains(option) {
                                    cached.push(option.clone());
                                }
                            }
                        }
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
        AppArg::new("--start-date", ArgType::DateVar),
        AppArg::new("--end-date", ArgType::DateVar),
        AppArg::new("--add-time-to-file", ArgType::Boolean),
        AppArg::new("--download-path", ArgType::String),
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

        let options = filter_options.remove(&f_name).unwrap_or_default();
        let arg_type = if options.is_empty() {
            ArgType::String
        } else {
            ArgType::List
        };
        let mut arg = AppArg::new(format!("--filter-{}", f_name), arg_type).depends_on(depends_map);
        if !options.is_empty() {
            arg = arg.options(options);
        }
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

fn normalize_filter_key(value: &str) -> String {
    value
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn sanitize_filename_component(value: &str) -> String {
    let mut sanitized = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
            sanitized.push('_');
        } else {
            sanitized.push(ch);
        }
    }

    if sanitized.chars().all(|ch| ch == '.' || ch == ' ') {
        return "report".to_string();
    }
    let sanitized = sanitized.replace("..", "_");
    let sanitized = sanitized.trim().trim_matches('.').trim_end_matches(' ');
    if sanitized.is_empty() {
        "report".to_string()
    } else {
        sanitized.to_string()
    }
}

async fn persist_discovered_filters(
    config_path: &Path,
    report_name: &str,
    discovered_filters: DiscoveredReportFilters,
    update_lock: &tokio::sync::Mutex<()>,
) -> Result<()> {
    let _guard = update_lock.lock().await;
    let latest_config_content = fs::read_to_string(config_path).await.with_context(|| {
        format!(
            "Failed to read Yasweb config at {:?} before updating filters",
            config_path
        )
    })?;
    let mut latest_config: YaswebConfig = serde_json::from_str(&latest_config_content)
        .with_context(|| {
            format!(
                "Refusing to overwrite invalid Yasweb config at {:?}",
                config_path
            )
        })?;

    let Some(report) = latest_config.reports.get_mut(report_name) else {
        return Ok(());
    };

    let mut updated_filters = false;
    for filter in discovered_filters.names {
        let filter_key = report
            .filters
            .keys()
            .find(|existing| normalize_filter_key(existing) == normalize_filter_key(&filter))
            .cloned()
            .unwrap_or(filter);
        if let std::collections::hash_map::Entry::Vacant(entry) = report.filters.entry(filter_key) {
            entry.insert(String::new());
            updated_filters = true;
        }
    }
    for (filter, options) in discovered_filters.options {
        let filter_key = report
            .filters
            .keys()
            .find(|existing| normalize_filter_key(existing) == normalize_filter_key(&filter))
            .cloned()
            .unwrap_or(filter);
        let cached = report.filter_options.entry(filter_key).or_default();
        for option in options {
            if !option.trim().is_empty() && !cached.contains(&option) {
                cached.push(option);
                updated_filters = true;
            }
        }
    }

    if updated_filters {
        info!(
            "Updating yasweb_config.json with newly discovered filters for {}...",
            report_name
        );
        let serialized = serde_json::to_string_pretty(&latest_config)
            .context("Failed to serialize Yasweb config after discovering filters")?;
        let path = config_path.to_path_buf();
        tokio::task::spawn_blocking(move || crm_tool::utils::atomic_write(&path, &serialized))
            .await
            .context("Failed to join Yasweb config write task")?
            .context("Failed to persist discovered Yasweb filters")?;
    }

    Ok(())
}

fn available_output_path(requested: &std::path::Path) -> std::path::PathBuf {
    if !requested.exists() {
        return requested.to_path_buf();
    }

    let parent = requested
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let stem = requested
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("report");
    let extension = requested
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("xlsx");
    for suffix in 1u32.. {
        let candidate = parent.join(format!("{stem} ({suffix}).{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("u32 output suffix space exhausted")
}

pub fn finalize_download(
    temp_dl_dir: &std::path::Path,
    final_out_dir: &std::path::Path,
    final_filename: &str,
) -> anyhow::Result<()> {
    let final_filename = sanitize_filename_component(final_filename);
    if !temp_dl_dir.is_dir() {
        anyhow::bail!("Download directory does not exist: {:?}", temp_dl_dir);
    }

    std::fs::create_dir_all(final_out_dir)
        .with_context(|| format!("Failed to create download directory {:?}", final_out_dir))?;

    // Serialize finalization across independent Yasweb processes sharing one output folder.
    let lock_path = final_out_dir.join(".yasweb-finalize.lock");
    let output_lock = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)
        .with_context(|| format!("Failed to open output lock {:?}", lock_path))?;
    fs2::FileExt::lock_exclusive(&output_lock)
        .with_context(|| format!("Failed to lock output directory {:?}", final_out_dir))?;

    let entries = std::fs::read_dir(temp_dl_dir)
        .with_context(|| format!("Failed to read download directory {:?}", temp_dl_dir))?;
    let mut report_files = Vec::new();
    for entry in entries {
        let entry = entry.with_context(|| format!("Failed to read entry in {:?}", temp_dl_dir))?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("xlsx")
                        || ext.eq_ignore_ascii_case("xls")
                        || ext.eq_ignore_ascii_case("csv")
                })
        {
            report_files.push(path);
        }
    }

    if report_files.is_empty() {
        anyhow::bail!(
            "No completed XLSX, XLS, or CSV report was found in {:?}",
            temp_dl_dir
        );
    }

    for path in &report_files {
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("xlsx");
        let mut final_name = final_filename.to_string();
        if !ext.eq_ignore_ascii_case("xlsx") {
            final_name = final_name.replace(".xlsx", &format!(".{}", ext));
        }
        let requested_out_file = final_out_dir.join(final_name);
        let out_file = available_output_path(&requested_out_file);

        tracing::info!("Moving downloaded file from {:?} to {:?}", path, out_file);
        if let Err(rename_error) = std::fs::rename(path, &out_file) {
            tracing::warn!(
                "Rename/move failed for {:?} -> {:?}: {}; falling back to copy",
                path,
                out_file,
                rename_error
            );
            if let Err(copy_error) = std::fs::copy(path, &out_file) {
                let _ = std::fs::remove_file(&out_file);
                return Err(copy_error).with_context(|| {
                    format!("Failed to move or copy report {:?} to {:?}", path, out_file)
                });
            }

            let source_len = std::fs::metadata(path)
                .with_context(|| format!("Failed to inspect source report {:?}", path))?
                .len();
            let destination_len = std::fs::metadata(&out_file)
                .with_context(|| format!("Failed to inspect copied report {:?}", out_file))?
                .len();
            if source_len != destination_len {
                let _ = std::fs::remove_file(&out_file);
                anyhow::bail!(
                    "Copied report size mismatch for {:?} -> {:?}",
                    path,
                    out_file
                );
            }

            std::fs::remove_file(path)
                .with_context(|| format!("Copied report but could not remove source {:?}", path))?;
        }
    }

    // Only remove the temporary directory after every report file has been moved successfully.
    std::fs::remove_dir_all(temp_dl_dir).with_context(|| {
        format!(
            "Reports moved, but failed to clean temporary directory {:?}",
            temp_dl_dir
        )
    })?;
    Ok(())
}
/// Runs the YAS Web scraper command-line application.
///
/// # Errors
///
/// Returns an error when the executable directory, configuration, logging setup,
/// filter input, configuration persistence, or report execution cannot be prepared
/// successfully.
///
/// # Examples
///
/// ```
/// # fn main() {
/// let args = ["yasweb", "--name", "sales"];
/// assert_eq!(args[1], "--name");
/// # }
/// ```
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

        let exe_dir = executable_dir()?;
        let config_path = config_path_opt.unwrap_or_else(|| exe_dir.join("yasweb_config.json"));
        if let InterceptResult::ExitSuccessfully =
            intercept_manifest(get_manifest(Some(config_path)))
        {
            return Ok(());
        }
    }

    let options = YaswebCliOptions::parse();
    // Accepted for compatibility with stored arguments; Runner owns date-range scheduling.
    let _legacy_monthly = options.monthly;

    let p = PathBuf::from(options.config);
    let exe_dir = executable_dir()?;
    let config_path = if p.is_absolute() { p } else { exe_dir.join(p) };

    let mut config: YaswebConfig =
        crm_tool::utils::load_or_create_config(&config_path, &YaswebConfig::default())?;

    config
        .validate()
        .context("Configuration validation failed")?;

    let _guard = setup_logging_with_levels(
        "yasweb",
        parse_log_level(&config.log_stdout_level)?,
        parse_log_level(&config.log_file_level)?,
    )?;

    let config_updated = config.finalize_runtime_fields();

    let active_report_names = options.name;
    let active_report_type_global = options.r#type.unwrap_or_default();

    let mut active_filters_global: HashMap<String, String> = HashMap::new();
    if let Some(filters_str) = options.filters {
        if !filters_str.trim().is_empty() {
            active_filters_global =
                serde_json::from_str(&filters_str).context("Failed to parse filters JSON")?;
        }
    }

    let start_date_str_global = options.start_date;
    let end_date_str_global = options.end_date;
    let add_time_to_file = options.add_time_to_file;
    let cli_download_path = options.download_path;

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
            let (
                existing_start_key,
                existing_end_key,
                existing_download_path,
                existing_filter_options,
            ) = if let Some(existing) = config.reports.get(active_report_name) {
                (
                    existing.start_date_key.clone(),
                    existing.end_date_key.clone(),
                    existing.download_path.clone(),
                    existing.filter_options.clone(),
                )
            } else {
                (None, None, None, HashMap::new())
            };

            let report_conf = ReportConfig {
                report_type: active_report_type_global.clone(),
                filters: active_filters_global.clone(),
                filter_options: existing_filter_options,
                start_date_key: existing_start_key,
                end_date_key: existing_end_key,
                download_path: existing_download_path,
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
        tokio::task::spawn_blocking({
            let path = config_path.clone();
            move || crm_tool::utils::atomic_write(&path, &content)
        })
        .await
        .context("Failed to join blocking task")?
        .context("Failed to write updated yasweb_config.json")?;
        info!("yasweb_config.json updated successfully.");
    }

    info!("Loaded config: {:#?}", config);
    info!("Loaded config for URL: {}", config.url);

    let mut report_futures = Vec::new();
    // Serialize config read-modify-write cycles so concurrent reports cannot lose discovered filters.
    let config_update_lock = Arc::new(tokio::sync::Mutex::new(()));
    // A zero limit means unlimited operations; positive values share one semaphore across reports.
    let operation_limit =
        (config.concurrency > 0).then(|| Arc::new(tokio::sync::Semaphore::new(config.concurrency)));
    if config.concurrency == 0 {
        info!("YASWEB operation concurrency limit: unlimited");
    } else {
        info!("YASWEB operation concurrency limit: {}", config.concurrency);
    }

    for active_report_name in active_report_names {
        let active_report_type = active_report_type_global.clone();
        let active_filters = active_filters_global.clone();
        let start_date_str = start_date_str_global.clone();
        let end_date_str = end_date_str_global.clone();
        let cli_download_path = cli_download_path.clone();

        let config = config.clone();
        let config_path_clone = config_path.clone();
        let config_update_lock = config_update_lock.clone();
        let operation_limit = operation_limit.clone();

        let active_report_name_clone_for_future = active_report_name.clone();

        // Push an async block into our futures list
        report_futures.push(async move {
            let active_report_name = active_report_name_clone_for_future;
            // Every process/report invocation gets its own Chrome profile and download folder.
            let run_id = format!(
                "{}_{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            );
            info!("YASWEB RUN INSTANCE: report='{}' run_id={}", active_report_name, run_id);
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

            // Runner owns scheduling; YASWEB executes one requested date range per invocation.
            let date_ranges: Vec<(String, String)> = vec![(String::new(), String::new())];

            let config_clone = config.clone();
            let active_report_name_clone = active_report_name.clone();
            let cli_download_path_clone = cli_download_path.clone();

            // Runner supplies the requested date range. YASWEB runs one browser operation per invocation.
            let i = 0;
            let (start_dt, end_dt) = (&date_ranges[0].0, &date_ranges[0].1);
                let mut tasks = Vec::new();
                    let config_task = config.clone();
                    let active_report_name_task = active_report_name.clone();
                    let active_report_type_task = active_report_type.clone();

                    let mut run_filters = active_filters.clone();
                    let report_conf_opt = config.reports.get(&active_report_name);

                    // Map the global --start-date and --end-date values into the respective internal report filters
                    // Map the requested dates into the report's configured filter formats.
                    let s_dt = {
                        // Re-format the input start date for this report.
                        if let (Some(st_str), Some(report_conf)) = (&start_date_str, report_conf_opt) {
                            if let Some(sk) = &report_conf.start_date_key {
                                if let Some(parsed) = crm_tool::utils::parse_flexible_date(st_str) {
                                    let parsed_dt = parsed.and_hms_opt(0, 0, 0).expect("midnight is valid");
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

                    let e_dt = {
                        // Re-format the input end date for this report.
                        if let (Some(ed_str), Some(report_conf)) = (&end_date_str, report_conf_opt) {
                            if let Some(ek) = &report_conf.end_date_key {
                                if let Some(parsed) = crm_tool::utils::parse_flexible_date(ed_str) {
                                    let parsed_dt = parsed.and_hms_opt(23, 59, 59).expect("23:59:59 is valid");
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

                    // Setup download dir for this tab
                    let date_suffix = if let Some(st) = start_date_str.clone() {
                        st
                    } else {
                        chrono::Local::now().format("%d-%m-%Y").to_string()
                    };

                    let safe_report_name = sanitize_filename_component(&active_report_name);
                    let safe_date_suffix = sanitize_filename_component(&date_suffix);
                    let temp_dl_dir = {
                        let mut dir = executable_dir()?;
                        dir.push(format!(
                            "yasweb_downloads_tmp_{}_{}_{}_{}",
                            safe_report_name, safe_date_suffix, i, run_id
                        ));
                        fs::create_dir_all(&dir)
                            .await
                            .with_context(|| format!("Failed to create temporary download directory {:?}", dir))?;
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

                    let active_report_name_for_dir = active_report_name.clone();
                    let run_id_for_browser = run_id.clone();

                    let operation_permit = if let Some(operation_limit) = &operation_limit {
                        tracing::trace!("Waiting for browser-operation permit: report={} range={}..{}", active_report_name_task, start_dt, end_dt);
                        Some(operation_limit.clone().acquire_owned().await
                            .context("Failed to acquire YASWEB operation concurrency permit")?)
                    } else {
                        None
                    };
                    tracing::trace!("Spawning task for range: {} to {}", start_dt, end_dt);
                    tasks.push(tokio::task::spawn_blocking(move || -> Result<(DiscoveredReportFilters, PathBuf, String)> {
                        let _operation_permit = operation_permit;
                        let task_started = std::time::Instant::now();
                        tracing::info!("YASWEB TASK START: index={} report='{}' date_suffix='{}'", i, active_report_name_task, date_suffix);
                        tracing::info!("Starting browser tab automation for task {}...", i);

                        let mut user_data_dir = crm_tool::utils::executable_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                        user_data_dir.push("yasweb_chrome_data");
                        user_data_dir.push(format!(
                            "session_{}_{}_{}_{}",
                            sanitize_filename_component(&active_report_name_for_dir),
                            sanitize_filename_component(&date_suffix),
                            i,
                            run_id_for_browser
                        ));

                        let args = vec![
                            std::ffi::OsStr::new("--start-maximized"),
                            std::ffi::OsStr::new("--disable-session-crashed-bubble"),
                            std::ffi::OsStr::new("--no-first-run"),
                            std::ffi::OsStr::new("--disable-infobars"),
                            std::ffi::OsStr::new("--skip-reopen-last-pages"),
                            std::ffi::OsStr::new("--disable-background-timer-throttling"),
                            std::ffi::OsStr::new("--disable-backgrounding-occluded-windows"),
                            std::ffi::OsStr::new("--disable-renderer-backgrounding"),
                        ];

                        let launch_options = headless_chrome::LaunchOptions::default_builder()
                            .headless(config_task.headless)
                            .idle_browser_timeout(std::time::Duration::MAX)
                            .user_data_dir(Some(user_data_dir.clone()))
                            .args(args)
                            .build()
                            .map_err(|e| anyhow::anyhow!("Failed to build launch options: {e}"));

                        let launch_options = match launch_options {
                            Ok(opts) => opts,
                            Err(e) => {
                                error!("YASWEB TASK FAILED: configure_launch_options task={} error={:#}", i, e);
                                return Err(e.context("Failed to configure browser launch options"));
                            }
                        };

                        let browser = match headless_chrome::Browser::new(launch_options) {
                            Ok(b) => Arc::new(b),
                            Err(e) => {
                                error!("YASWEB TASK FAILED: launch_browser task={} error={:#}", i, e);
                                return Err(anyhow::anyhow!("Failed to launch browser: {e}"));
                            }
                        };

                        let res = match run_browser_tab(
                            browser.clone(),


                            &config_task,
                            &active_report_name_task,
                            &active_report_type_task,
                            &run_filters,
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
                                error!("YASWEB TASK FAILED: browser_automation task={} elapsed_ms={} error={:#}", i, task_started.elapsed().as_millis(), e);
                                eprintln!("Browser automation failed for task {}: {:#}", i, e);
                                drop(browser);
                                let _ = std::fs::remove_dir_all(&user_data_dir);
                                return Err(e.context(format!("Browser automation failed for task {}", i)));
                            }
                        };

                        drop(browser);
                        let _ = std::fs::remove_dir_all(&user_data_dir);
                        tracing::info!("YASWEB TASK END: status=SUCCESS task={} elapsed_ms={} discovered_filters={} dropdown_lists={}", i, task_started.elapsed().as_millis(), res.names.len(), res.options.len());
                        Ok((res, temp_dl_dir_clone, final_filename))
                    }));

                let results = futures_util::future::join_all(tasks).await;

                for task_result in results {
                    let (discovered_filters, temp_dl_dir, final_filename) = match task_result {
                        Ok(Ok(result)) => result,
                        Ok(Err(e)) => return Err(e.context("Yasweb browser task failed")),
                        Err(e) => return Err(anyhow::anyhow!("Yasweb browser task panicked or was cancelled: {e}")),
                    };
                    let final_out_dir = if let Some(ref path) = cli_download_path_clone {
                        PathBuf::from(path)
                    } else if let Some(report) = config_clone.reports.get(&active_report_name_clone) {
                        if let Some(ref path) = report.download_path {
                            PathBuf::from(path)
                        } else if let Some(ref path) = config_clone.download_path {
                            PathBuf::from(path)
                        } else {
                            let mut dir = executable_dir()?;
                            dir.push("downloads");
                            dir
                        }
                    } else if let Some(ref path) = config_clone.download_path {
                        PathBuf::from(path)
                    } else {
                        let mut dir = executable_dir()?;
                        dir.push("downloads");
                        dir
                    };

                    let finalization = tokio::task::spawn_blocking(move || {
                        finalize_download(&temp_dl_dir, &final_out_dir, &final_filename)
                    })
                    .await
                    .context("Yasweb download finalization task failed to join")?;
                    finalization.with_context(|| {
                        format!("Failed to finalize Yasweb report '{}'", active_report_name_clone)
                    })?;

                    if !discovered_filters.names.is_empty() || !discovered_filters.options.is_empty() {
                        if let Err(e) = persist_discovered_filters(
                            &config_path_clone,
                            &active_report_name_clone,
                            discovered_filters,
                            &config_update_lock,
                        ).await {
                            // Filter discovery is optional; never overwrite a config that could not be read or parsed.
                            tracing::warn!(
                                "Could not persist discovered filters for '{}': {:#}",
                                active_report_name_clone,
                                e
                            );
                        }
                    }
                }
            Ok::<(), anyhow::Error>(())
        });
    }

    // Reports may run concurrently, while the shared semaphore caps active browser operations.
    let report_results = futures_util::future::join_all(report_futures).await;
    for result in report_results {
        if let Err(e) = result {
            error!("YASWEB REPORT FAILED: {:#}", e);
            return Err(e);
        }
    }
    info!("YASWEB PROCESS END: status=SUCCESS");
    Ok(())
}

#[cfg(test)]
mod tests_yasweb_finalization {
    use super::*;
    use std::fs::File;
    use tokio::sync::Mutex as AsyncMutex;

    #[test]
    fn filename_components_cannot_escape_their_directory() {
        assert_eq!(
            sanitize_filename_component("../../outside\\file.csv"),
            "____outside_file.csv"
        );
        assert_eq!(
            sanitize_filename_component("C:\\temp\\report.csv"),
            "C__temp_report.csv"
        );
        assert_eq!(
            sanitize_filename_component("report 🚀 data.xlsx"),
            "report 🚀 data.xlsx"
        );
        assert_eq!(sanitize_filename_component("..."), "report");
    }

    #[test]
    fn manifest_exposes_cached_dropdown_options_as_a_list() {
        let root = tempfile::tempdir().unwrap();
        let config_path = root.path().join("yasweb_config.json");
        let config = YaswebConfig {
            url: "https://example.test".to_string(),
            username: "test".to_string(),
            reports: HashMap::from([(
                "Appointments".to_string(),
                ReportConfig {
                    report_type: "Report Manager".to_string(),
                    filters: HashMap::from([("Hospital".to_string(), String::new())]),
                    filter_options: HashMap::from([(
                        "Hospital".to_string(),
                        vec!["Fakeeh".to_string(), "Neom".to_string()],
                    )]),
                    start_date_key: None,
                    end_date_key: None,
                    download_path: None,
                },
            )]),
            ..YaswebConfig::default()
        };
        std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

        let manifest = get_manifest(Some(config_path));
        let hospital = manifest
            .arguments
            .iter()
            .find(|argument| argument.name == "--filter-Hospital")
            .expect("Hospital filter should be exposed");

        assert_eq!(hospital.arg_type, crm_tool::manifest::ArgType::List);
        assert_eq!(
            hospital.options.as_ref().unwrap(),
            &vec!["Fakeeh".to_string(), "Neom".to_string()]
        );
    }

    #[tokio::test]
    async fn concurrent_filter_updates_preserve_both_reports() {
        let root = tempfile::tempdir().unwrap();
        let config_path = root.path().join("yasweb_config.json");
        let config = YaswebConfig {
            url: "https://example.test".to_string(),
            username: "test".to_string(),
            reports: std::collections::HashMap::from([
                (
                    "first".to_string(),
                    ReportConfig {
                        report_type: "Standard Report".to_string(),
                        filters: HashMap::from([("HOSPITAL".to_string(), String::new())]),
                        filter_options: std::collections::HashMap::new(),
                        start_date_key: None,
                        end_date_key: None,
                        download_path: None,
                    },
                ),
                (
                    "second".to_string(),
                    ReportConfig {
                        report_type: "Standard Report".to_string(),
                        filters: std::collections::HashMap::new(),
                        filter_options: std::collections::HashMap::new(),
                        start_date_key: None,
                        end_date_key: None,
                        download_path: None,
                    },
                ),
            ]),
            ..YaswebConfig::default()
        };
        std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
        let lock = Arc::new(AsyncMutex::new(()));

        let (first, second) = tokio::join!(
            persist_discovered_filters(
                &config_path,
                "first",
                DiscoveredReportFilters {
                    names: vec!["filter_a".to_string(), "Hospital".to_string()],
                    options: HashMap::from([(
                        "Hospital".to_string(),
                        vec!["Fakeeh".to_string(), "Neom".to_string()],
                    )]),
                },
                &lock,
            ),
            persist_discovered_filters(
                &config_path,
                "second",
                DiscoveredReportFilters {
                    names: vec!["filter_b".to_string()],
                    options: HashMap::new()
                },
                &lock
            ),
        );
        first.unwrap();
        second.unwrap();

        let saved: YaswebConfig =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert!(saved.reports["first"].filters.contains_key("filter_a"));
        assert!(saved.reports["first"].filters.contains_key("HOSPITAL"));
        assert!(!saved.reports["first"].filters.contains_key("Hospital"));
        assert_eq!(
            saved.reports["first"].filter_options["HOSPITAL"],
            vec!["Fakeeh".to_string(), "Neom".to_string()]
        );
        assert!(saved.reports["second"].filters.contains_key("filter_b"));
    }

    #[tokio::test]
    async fn invalid_config_is_not_replaced_when_persisting_discovered_filters() {
        let root = tempfile::tempdir().unwrap();
        let config_path = root.path().join("yasweb_config.json");
        let invalid = "{ invalid json";
        std::fs::write(&config_path, invalid).unwrap();
        let lock = AsyncMutex::new(());

        assert!(persist_discovered_filters(
            &config_path,
            "first",
            DiscoveredReportFilters {
                names: vec!["filter_a".to_string()],
                options: HashMap::new()
            },
            &lock,
        )
        .await
        .is_err());
        assert_eq!(std::fs::read_to_string(&config_path).unwrap(), invalid);
    }
    use std::io::Write;

    #[test]
    fn finalize_download_keeps_sanitized_filename_inside_output_directory() {
        let root = tempfile::tempdir().unwrap();
        let src_dir = root.path().join("source");
        let dst_dir = root.path().join("destination");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("report.xlsx"), b"safe report").unwrap();

        finalize_download(&src_dir, &dst_dir, "../outside.xlsx").unwrap();

        let output_files = std::fs::read_dir(&dst_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(output_files.len(), 1, "expected exactly one output file");
        assert_eq!(output_files[0].parent(), Some(dst_dir.as_path()));
        assert_ne!(
            output_files[0].file_name().and_then(|name| name.to_str()),
            Some("outside.xlsx"),
            "the output filename must not escape sanitization"
        );
        assert!(!root.path().join("outside.xlsx").exists());
    }

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
        // A missing download directory is a failed report finalization, not success.
        assert!(finalize_download(&src_dir, &dst_dir, "final_report.xlsx").is_err());
        assert!(!dst_dir.join("final_report.xlsx").exists());
    }

    #[test]
    fn test_finalize_download_preserves_source_when_destination_cannot_be_written() {
        let root = tempfile::tempdir().unwrap();
        let src_dir = root.path().join("source");
        let dst_file = root.path().join("destination_is_a_file");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("report.xlsx"), b"report contents").unwrap();
        std::fs::write(&dst_file, b"not a directory").unwrap();

        assert!(finalize_download(&src_dir, &dst_file, "final.xlsx").is_err());
        assert!(src_dir.join("report.xlsx").exists());
        assert_eq!(
            std::fs::read(src_dir.join("report.xlsx")).unwrap(),
            b"report contents"
        );
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

        let original = std::fs::read_to_string(&dst_file).unwrap();
        let collision_copy = std::fs::read_to_string(dst_dir.join("final (1).xlsx")).unwrap();
        assert_eq!(original, "old data");
        assert_eq!(collision_copy, "new data");
        std::fs::remove_dir_all(&dst_dir).unwrap();
    }
}
