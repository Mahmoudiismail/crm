use crate::tasker::config::CrmOpenSohailConfig;
use anyhow::Result;
use std::collections::HashMap;
use tracing::{error, info, warn};

use super::models::{EnrichedDataset, EnrichedRow, ExtractedSlicerDataset, TeamMappingInfo};

fn calculate_date_ranges(
    config: &CrmOpenSohailConfig,
    current_month_dt: &str,
) -> HashMap<String, (Option<chrono::NaiveDateTime>, Option<chrono::NaiveDateTime>)> {
    let mut branch_date_ranges = HashMap::new();

    let output_file_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.dashboard_config.output_file);
    if let Ok(mut output_rdr) = csv::ReaderBuilder::new().from_path(&output_file_path) {
        if let Ok(headers) = output_rdr.headers() {
            let created_at_idx = headers
                .iter()
                .position(|h| h.trim().eq_ignore_ascii_case("Created At"));
            let branch_idx = headers
                .iter()
                .position(|h| h.trim().eq_ignore_ascii_case("Branch"));

            if let (Some(c_idx), Some(b_idx)) = (created_at_idx, branch_idx) {
                for record in output_rdr.records().flatten() {
                    if let (Some(created_val), Some(branch_val)) =
                        (record.get(c_idx), record.get(b_idx))
                    {
                        if let Some(dt) = crate::tasker::csv_task::parse_created_at(created_val) {
                            // Only include this date if it is not in the current month
                            if dt.format("%b-%Y").to_string() != current_month_dt {
                                let branch = branch_val.trim().to_string();
                                let entry =
                                    branch_date_ranges.entry(branch).or_insert((None, None));
                                if entry.0.is_none_or(|m| dt < m) {
                                    entry.0 = Some(dt);
                                }
                                if entry.1.is_none_or(|m| dt > m) {
                                    entry.1 = Some(dt);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    branch_date_ranges
}

fn apply_date_ranges(
    extracted_data: &mut [ExtractedSlicerDataset],
    branch_date_ranges: &HashMap<
        String,
        (Option<chrono::NaiveDateTime>, Option<chrono::NaiveDateTime>),
    >,
    current_month_dt: &str,
) {
    // Apply the accurate CSV date range per branch
    for dataset in extracted_data {
        // Any branch with a combined date range (starting with "from (") should be overridden
        // except if it happens to be explicitly the current month dt.
        if dataset.month.contains("from (") && dataset.month != current_month_dt {
            let mut matching_branch_range = None;
            // The CSV might contain variations of the branch name. Let's do a case-insensitive check.
            for (csv_branch, (min_date, max_date)) in branch_date_ranges {
                if csv_branch.eq_ignore_ascii_case(dataset.branch.trim()) {
                    matching_branch_range = Some((*min_date, *max_date));
                    break;
                }
            }

            if let Some((Some(min), Some(max))) = matching_branch_range {
                let min_month = min.format("%b").to_string();
                let max_month = max.format("%b-%Y").to_string();

                let computed_month_range =
                    if min.format("%b-%Y").to_string() == max.format("%b-%Y").to_string() {
                        min.format("%b-%Y").to_string()
                    } else {
                        format!("{} to {}", min_month, max_month)
                    };
                dataset.month = computed_month_range;
            }
        }
    }
}

fn resolve_team_mapping(config: &CrmOpenSohailConfig) -> Result<HashMap<String, TeamMappingInfo>> {
    let team_mapping_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.team_mapping_file);
    if !team_mapping_path.exists() {
        error!(
            "Team mapping file not found at: {}",
            team_mapping_path.display()
        );
        anyhow::bail!("Team mapping file not found");
    }

    let mut team_to_info = HashMap::new();

    let file = std::fs::File::open(&team_mapping_path)?;
    let mut rdr = crate::utils::build_csv_reader_from_reader(file);

    // We expect columns like "Team Name", "Owner" (or Receiver Name), "To Emails"
    // We strictly prefer "owner_name" and "owner_email" over legacy "receiver_name" and "to_emails".

    let headers = rdr.headers()?.clone();
    let mut team_idx = None;
    let mut owner_name_idx = None;
    let mut receiver_name_idx = None;
    let mut owner_email_idx = None;
    let mut to_emails_idx = None;
    let mut is_shared_idx = None;

    for (i, h) in headers.iter().enumerate() {
        let h_lower = h.trim().to_lowercase();
        if (h_lower == "team name" || h_lower == "team" || h_lower.contains("team"))
            && team_idx.is_none()
        {
            team_idx = Some(i);
        } else if h_lower == "owner_name" || h_lower == "owner" || h_lower == "oul" {
            owner_name_idx = Some(i);
        } else if h_lower == "receiver name"
            || h_lower == "receiver_name"
            || h_lower.contains("receiver")
        {
            receiver_name_idx = Some(i);
        } else if h_lower == "owner_email" || h_lower == "email_to" || h_lower == "owner email" {
            owner_email_idx = Some(i);
        } else if (h_lower == "to emails"
            || h_lower == "to_emails"
            || h_lower == "email"
            || h_lower.contains("email"))
            && owner_email_idx.is_none()
        {
            to_emails_idx = Some(i);
        } else if h_lower == "is_shared"
            || h_lower == "is_shared_across_branches"
            || h_lower == "shared"
            || h_lower == "is_shared_team"
        {
            is_shared_idx = Some(i);
        }
    }

    for record in rdr.records().filter_map(|r| r.ok()) {
        if let Some(t_idx) = team_idx {
            let team_name = record.get(t_idx).unwrap_or("").trim().to_lowercase();
            if team_name.is_empty() {
                continue;
            }

            let owner = owner_name_idx
                .and_then(|idx| record.get(idx))
                .filter(|s| !s.trim().is_empty())
                .or_else(|| receiver_name_idx.and_then(|idx| record.get(idx)))
                .unwrap_or("")
                .trim()
                .to_string();

            let email = owner_email_idx
                .and_then(|idx| record.get(idx))
                .filter(|s| !s.trim().is_empty())
                .or_else(|| to_emails_idx.and_then(|idx| record.get(idx)))
                .unwrap_or("")
                .trim()
                .to_string();

            let is_shared = is_shared_idx
                .and_then(|idx| record.get(idx))
                .map(|s| {
                    let s_low = s.trim().to_lowercase();
                    s_low == "true" || s_low == "1" || s_low == "yes" || s_low == "y"
                })
                .unwrap_or(false);

            team_to_info.insert(
                team_name,
                TeamMappingInfo {
                    owner_name: owner,
                    owner_email: email,
                    is_shared,
                },
            );
        }
    }

    info!(
        "Loaded {} team mappings for OUL enrichment.",
        team_to_info.len()
    );

    Ok(team_to_info)
}

fn enrich_dataset(
    extracted_data: Vec<ExtractedSlicerDataset>,
    team_to_info: &HashMap<String, TeamMappingInfo>,
    fallback_oul_text: &str,
) -> Vec<EnrichedDataset> {
    let mut final_datasets: Vec<EnrichedDataset> = Vec::new();

    for mut dataset in extracted_data {
        let mut enriched_rows = Vec::new();

        let is_main_branch = dataset
            .branch
            .trim()
            .eq_ignore_ascii_case("Dr. Soliman Fakeeh Hospital Jeddah")
            || dataset.branch.trim().eq_ignore_ascii_case("DSFH")
            || dataset
                .branch
                .trim()
                .to_lowercase()
                .contains("soliman fakeeh hospital");

        for row in dataset.data.drain(..) {
            let team_lower = row.team.trim().to_lowercase();
            let team_info = team_to_info.get(&team_lower);

            let mut oul = match team_info {
                Some(info) => {
                    if info.is_shared || is_main_branch {
                        match (&info.owner_name, &info.owner_email) {
                            (o, e) if !o.is_empty() && !e.is_empty() => {
                                format!("<a href=\"mailto:{}\">{}</a>", e, o)
                            }
                            (o, _) if !o.is_empty() => o.to_string(),
                            _ => fallback_oul_text.to_string(),
                        }
                    } else {
                        fallback_oul_text.to_string()
                    }
                }
                None => {
                    warn!("Missing team mapping for: {}", row.team);
                    fallback_oul_text.to_string()
                }
            };

            if row.open == 0.0 {
                oul = String::new();
            }

            enriched_rows.push(EnrichedRow {
                team: row.team,
                closed: row.closed,
                open: row.open,
                perc_closed: row.perc_closed,
                perc_open: row.perc_open,
                grand_total: row.grand_total,
                oul,
            });
        }

        if !enriched_rows.is_empty() {
            final_datasets.push(EnrichedDataset {
                branch: dataset.branch,
                month: dataset.month,
                data: enriched_rows,
            });
        }
    }

    final_datasets
}

pub fn process_extracted_data(
    config: &CrmOpenSohailConfig,
    mut extracted_data: Vec<ExtractedSlicerDataset>,
) -> Result<Vec<EnrichedDataset>> {
    let current_month_dt = chrono::Local::now().format("%b-%Y").to_string();

    let branch_date_ranges = calculate_date_ranges(config, &current_month_dt);
    apply_date_ranges(&mut extracted_data, &branch_date_ranges, &current_month_dt);

    let team_to_info = resolve_team_mapping(config)?;
    let fallback_oul_text = config.fallback_oul.clone().unwrap_or_default();

    let final_datasets = enrich_dataset(extracted_data, &team_to_info, &fallback_oul_text);

    Ok(final_datasets)
}
