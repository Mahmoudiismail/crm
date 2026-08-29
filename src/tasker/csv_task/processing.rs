use crate::tasker::csv_task::models::{CsvAnalysisParams, UserInfo};
use crate::tasker::csv_task::parse_created_at;
use crate::tasker::csv_task::parse_start_date;
use crate::tasker::csv_task::reader::TicketCsvReader;
use csv::StringRecord;
use std::collections::{HashMap, HashSet};
use tracing::info;

#[derive(Debug)]
pub struct ProcessResult {
    pub records: Vec<(String, StringRecord)>,
    pub headers: Option<StringRecord>,
    pub total_filtered_rows: usize,
    pub total_deduped_rows: usize,
}

pub fn process_files(
    target_files: Vec<std::path::PathBuf>,
    params: &CsvAnalysisParams<'_>,
    assignee_map: &HashMap<String, UserInfo>,
    assignment_map: &HashMap<(String, String, String), String>,
) -> anyhow::Result<ProcessResult> {
    let mut all_records = Vec::new();
    let mut total_filtered_rows = 0;
    let mut total_deduped_rows = 0;
    let mut seen_tickets = HashSet::new();
    let mut out_headers = None;

    let exclude_branches: HashSet<String> = params
        .exclude_branches
        .iter()
        .map(|s| s.trim().to_lowercase())
        .collect();
    let exclude_categories: HashSet<String> = params
        .exclude_categories
        .iter()
        .map(|s| s.trim().to_lowercase())
        .collect();

    let parsed_start_date = params.start_date.and_then(parse_created_at);
    let filter_start_date_dt = params.start_date.and_then(parse_start_date);

    for file_path in target_files {
        info!("Processing file: {}", file_path.display());
        let mut ticket_reader = match TicketCsvReader::new(&file_path)? {
            Some(r) => r,
            None => continue,
        };
        let headers: csv::StringRecord = ticket_reader.headers.clone();

        let mut assignee_idx = None;
        let mut type_idx = None;
        let mut subtype_idx = None;
        let mut cat_idx = None;
        let mut ticket_id_idx = None;
        let mut branch_idx = None;
        let mut created_at_idx = None;

        for (i, h) in headers.iter().enumerate() {
            let h_trim = h.trim();
            if h_trim == "Assignee" {
                assignee_idx = Some(i);
            } else if h_trim == "Ticket Type" {
                type_idx = Some(i);
            } else if h_trim == "Ticket Sub-Type" {
                subtype_idx = Some(i);
            } else if h_trim == "Ticket Category" {
                cat_idx = Some(i);
            } else if h_trim == "Ticket Id" {
                ticket_id_idx = Some(i);
            } else if h_trim == "Branch" {
                branch_idx = Some(i);
            } else if h_trim.eq_ignore_ascii_case("created at")
                || h_trim.eq_ignore_ascii_case("creation date")
            {
                created_at_idx = Some(i);
            }
        }

        if out_headers.is_none() {
            let mut h = headers.clone();
            h.push_field("Position");
            h.push_field("team");
            h.push_field("Is Exception");
            h.push_field("Month");
            out_headers = Some(h);
        }

        let mut new_record = StringRecord::new();
        for result in ticket_reader.records() {
            let mut record: csv::StringRecord = result?;
            let mut is_exception_val = "No";

            let mut month_val = String::new();
            if let Some(created_idx) = created_at_idx {
                let created_val = record.get(created_idx).unwrap_or("").trim();
                if let Some(dt) = parse_created_at(created_val) {
                    month_val = dt.format("%b-%Y").to_string();
                }
            }

            if let Some(start_dt) = parsed_start_date {
                if let Some(created_idx) = created_at_idx {
                    let created_val = record.get(created_idx).unwrap_or("").trim();
                    if let Some(dt) = parse_created_at(created_val) {
                        if dt < start_dt {
                            total_filtered_rows += 1;
                            continue;
                        }
                    }
                }
            }

            new_record.clear();
            for (i, field) in record.iter().enumerate() {
                if Some(i) == assignee_idx {
                    new_record.push_field(field.trim());
                } else if Some(i) == type_idx || Some(i) == subtype_idx || Some(i) == cat_idx {
                    if field.contains('_') {
                        new_record.push_field(&field.replace('_', " "));
                    } else {
                        new_record.push_field(field);
                    }
                } else {
                    new_record.push_field(field);
                }
            }

            std::mem::swap(&mut record, &mut new_record);

            let ticket_id_val = ticket_id_idx.and_then(|idx| record.get(idx)).unwrap_or("");
            if seen_tickets.contains(ticket_id_val) {
                total_deduped_rows += 1;
                continue;
            }
            let ticket_id_val_owned = ticket_id_val.to_string();
            seen_tickets.insert(ticket_id_val_owned.clone());

            let branch_val = branch_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .trim()
                .to_lowercase();

            let t_type = type_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .to_uppercase();
            let t_subtype = subtype_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .to_uppercase();
            let t_cat = cat_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .to_uppercase();
            let assignee = assignee_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .to_uppercase();

            let team2 = assignment_map.get(&(t_cat, t_type, t_subtype)).cloned();

            let (position, mut team) = if let Some(user_info) = assignee_map.get(&assignee) {
                let pos = if user_info.positions.is_empty() {
                    None
                } else if let Some(t2) = &team2 {
                    if user_info.positions.contains(t2) {
                        Some(t2.clone())
                    } else {
                        user_info.first_position.clone()
                    }
                } else {
                    user_info.first_position.clone()
                };

                let tm = pos.clone().or(team2.clone());
                (pos, tm)
            } else {
                (None, team2.clone())
            };

            if exclude_branches.contains(&branch_val) {
                total_filtered_rows += 1;
                continue;
            }

            let cat_val = cat_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("")
                .trim()
                .to_lowercase();

            if exclude_categories.contains(&cat_val) {
                let mut matches_exception = false;
                if let Some(exceptions) = params.category_exceptions {
                    for exc in exceptions {
                        if exc.category.trim().to_lowercase() == cat_val {
                            let branch_matches = exc.branch.as_ref().is_none_or(|b| {
                                let b_trim = b.trim().to_lowercase();
                                b_trim.is_empty()
                                    || b_trim == branch_val
                                    || b_trim.contains(&branch_val)
                                    || branch_val.contains(&b_trim)
                            });

                            if branch_matches {
                                matches_exception = true;
                                if let Some(t) = exc.team.as_ref() {
                                    if !t.trim().is_empty() {
                                        team = Some(t.trim().to_string());
                                    }
                                }
                                break;
                            }
                        }
                    }
                }

                if !matches_exception {
                    total_filtered_rows += 1;
                    continue;
                }

                is_exception_val = "Yes";
            }

            if let Some(start_dt) = filter_start_date_dt {
                if let Some(created_dt) = created_at_idx
                    .and_then(|idx| record.get(idx))
                    .and_then(parse_created_at)
                {
                    if created_dt < start_dt {
                        total_filtered_rows += 1;
                        continue;
                    }
                }
            }

            record.push_field(position.as_deref().unwrap_or(""));
            record.push_field(team.as_deref().unwrap_or(""));
            record.push_field(is_exception_val);
            record.push_field(&month_val);

            all_records.push((ticket_id_val_owned, record));
        }
    }

    all_records.sort_by(|a, b| {
        let a_num = a.0.parse::<u64>().unwrap_or(0);
        let b_num = b.0.parse::<u64>().unwrap_or(0);
        if a_num > 0 && b_num > 0 {
            a_num.cmp(&b_num)
        } else {
            a.0.cmp(&b.0)
        }
    });

    Ok(ProcessResult {
        records: all_records,
        headers: out_headers,
        total_filtered_rows,
        total_deduped_rows,
    })
}
