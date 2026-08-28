use crate::tasker::config::CrmOpenSohailConfig;
use crate::tasker::crm_open_sohail::models::{
    EnrichedDataset, EnrichedRow, ExtractedSlicerDataset, TeamMappingInfo,
};
use anyhow::Result;
use tracing::{error, info, warn};

pub fn process_data(
    config: &CrmOpenSohailConfig,
    extracted_data: Vec<ExtractedSlicerDataset>,
    _branch_date_ranges: &std::collections::HashMap<
        String,
        (Option<chrono::NaiveDateTime>, Option<chrono::NaiveDateTime>),
    >,
) -> Result<Vec<EnrichedDataset>> {
    // Step 5: Process Data & Enrich OUL Column
    let team_mapping_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.team_mapping_file);
    if !team_mapping_path.exists() {
        error!(
            "Team mapping file not found at: {}",
            team_mapping_path.display()
        );
        anyhow::bail!("Team mapping file not found");
    }

    let mut team_to_info = std::collections::HashMap::new();

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

    let fallback_oul_text = config.fallback_oul.clone().unwrap_or_default();

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
                            _ => fallback_oul_text.clone(),
                        }
                    } else {
                        fallback_oul_text.clone()
                    }
                }
                None => {
                    warn!("Missing team mapping for: {}", row.team);
                    fallback_oul_text.clone()
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

    Ok(final_datasets)
}
