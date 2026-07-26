use crate::tasker::config::EmailConfig;
use crate::tasker::email::message::{TeamMapping, TicketRow};
use crate::tasker::email::utils::title_case;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use tracing::{error, info};

pub fn load_team_mappings(
    team_mapping_path: &std::path::Path,
) -> Result<HashMap<String, TeamMapping>> {
    let mut team_maps = HashMap::new();
    let mapping_file = File::open(team_mapping_path).context(format!(
        "Failed to open team mapping file: {}",
        team_mapping_path.display()
    ))?;
    let mut map_rdr = crate::utils::build_csv_reader_from_reader(mapping_file);

    for result in map_rdr.deserialize::<TeamMapping>() {
        match result {
            Ok(mapping) => {
                tracing::trace!("Loaded team mapping: {:?}", mapping);
                team_maps.insert(mapping.team_name.trim().to_lowercase(), mapping);
            }
            Err(e) => {
                error!("Failed to parse row in team mapping file: {}", e);
            }
        }
    }
    info!("Loaded {} team mappings.", team_maps.len());
    Ok(team_maps)
}

#[derive(Debug)]
pub struct Buckets {
    pub per_team: HashMap<String, Vec<TicketRow>>,
    pub per_branch: HashMap<String, Vec<TicketRow>>,
    pub call_center: Vec<TicketRow>,
}

pub fn group_tickets_into_buckets(
    ticket_rows: Vec<TicketRow>,
    config: &EmailConfig,
    only_call_center: bool,
    effective_send_exceptions: bool,
) -> Buckets {
    let send_per_team_all_branches: HashSet<String> = config
        .send_per_team_all_branches
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    let send_per_branch_branches: HashSet<String> = config
        .send_per_branch_branches
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    let send_per_team_branches: HashSet<String> = config
        .send_per_team_branches
        .as_ref()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|s| s.to_lowercase())
        .collect();

    let send_cc = only_call_center || config.send_call_center.unwrap_or(false);

    let mut per_team_buckets: HashMap<String, Vec<TicketRow>> = HashMap::new();
    let mut per_branch_buckets: HashMap<String, Vec<TicketRow>> = HashMap::new();
    let mut call_center_bucket: Vec<TicketRow> = Vec::new();

    for row in ticket_rows {
        let b_low = row.branch.to_lowercase();
        let t_low = row.team.to_lowercase();

        let is_cc = t_low == "call center";

        let allowed_branch = send_per_branch_branches.contains(&b_low);
        let normalized_team = title_case(&t_low);

        if effective_send_exceptions {
            per_team_buckets
                .entry(normalized_team)
                .or_default()
                .push(row);
        } else if is_cc {
            if send_cc {
                call_center_bucket.push(row);
            }
        } else if !only_call_center {
            if send_per_team_all_branches.contains(&t_low) {
                per_team_buckets
                    .entry(normalized_team)
                    .or_default()
                    .push(row);
            } else if allowed_branch {
                per_branch_buckets
                    .entry(row.branch.clone())
                    .or_default()
                    .push(row);
            } else if send_per_team_branches.contains(&b_low) {
                per_team_buckets
                    .entry(normalized_team)
                    .or_default()
                    .push(row);
            }
        }
    }

    Buckets {
        per_team: per_team_buckets,
        per_branch: per_branch_buckets,
        call_center: call_center_bucket,
    }
}

pub fn resolve_recipients(
    bucket_name: &str,
    mapping: Option<&TeamMapping>,
    config: &EmailConfig,
    effective_send_exceptions: bool,
    exception_teams: &HashSet<String>,
) -> (String, String) {
    let mapped_to = mapping
        .and_then(|m| m.to_emails.clone())
        .unwrap_or_default();

    if mapped_to.trim().is_empty() {
        (config.default_to_email.clone(), String::new())
    } else {
        let mapped_cc = mapping.and_then(|m| m.cc.clone()).unwrap_or_default();
        let ccs = if bucket_name.eq_ignore_ascii_case("call center")
            || effective_send_exceptions
            || exception_teams.contains(&bucket_name.to_lowercase())
        {
            vec![mapped_cc]
        } else {
            vec![
                config.initial_cc.clone(),
                mapped_cc,
                config.ending_cc.clone(),
            ]
        }
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>()
        .join(";");
        (mapped_to, ccs)
    }
}
