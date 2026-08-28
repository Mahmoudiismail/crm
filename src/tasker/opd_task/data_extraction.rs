use crate::tasker::config::OpdAnalysisConfig;
use crate::tasker::opd_task::file_discovery::FileInfo;
use calamine::{Data, DataType, Reader};
use chrono::{NaiveDate, Timelike};
use std::collections::HashMap;
use tracing::warn;

pub fn extract_new_data(
    config: &OpdAnalysisConfig,
    new_files: Vec<FileInfo>,
) -> HashMap<(NaiveDate, String), i64> {
    let mut new_data: HashMap<(NaiveDate, String), i64> = HashMap::new(); // (Date, "HH:00") -> Total Patients Seen

    for fi in new_files {
        let mut total_seen = 0;
        let mut workbook = match calamine::open_workbook_auto(&fi.path) {
            Ok(w) => w,
            Err(e) => {
                warn!("Failed to open workbook {:?}: {}", fi.path, e);
                continue;
            }
        };

        if let Ok(range) = workbook.worksheet_range("MIS_Report") {
            if range.rows().count() < 5 {
                continue;
            }
            let headers: Vec<String> = range
                .rows()
                .nth(4)
                .unwrap()
                .iter()
                .map(|c| c.to_string())
                .collect();
            let total_slot_idx = headers.iter().position(|r| r == "Total Slot");
            let special_idx = headers.iter().position(|r| r == "Speciality");
            let emp_name_idx = headers.iter().position(|r| r == "Emp Name");
            let dept_idx = headers.iter().position(|r| r == "Dept");
            let patient_seen_idx = headers.iter().position(|r| r == "Total Patient Seen");

            for row in range.rows().skip(5) {
                let total_slot_val = total_slot_idx.and_then(|idx| row.get(idx));
                let is_total_slot_valid = match total_slot_val {
                    Some(v) if v.is_empty() => false,
                    Some(v) => {
                        let n = v
                            .get_int()
                            .unwrap_or_else(|| v.get_float().unwrap_or(0.0) as i64);
                        n != 0
                    }
                    None => false,
                };
                if !is_total_slot_valid {
                    continue;
                }

                let special_val = special_idx
                    .and_then(|idx| row.get(idx))
                    .unwrap_or(&Data::Empty)
                    .to_string();
                let special_val_lower = special_val.to_lowercase();
                let has_prefix = config
                    .exclude_speciality_prefixes
                    .iter()
                    .any(|p| special_val_lower.starts_with(&p.to_lowercase()));
                if has_prefix {
                    continue;
                }

                if config
                    .exclude_specialities
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(&special_val))
                {
                    continue;
                }

                let emp_name = emp_name_idx
                    .and_then(|idx| row.get(idx))
                    .unwrap_or(&Data::Empty)
                    .to_string();
                if config
                    .exclude_emp_names
                    .iter()
                    .any(|e| e.eq_ignore_ascii_case(&emp_name))
                {
                    continue;
                }

                let dept = dept_idx
                    .and_then(|idx| row.get(idx))
                    .unwrap_or(&Data::Empty)
                    .to_string();
                if config
                    .exclude_depts
                    .iter()
                    .any(|d| d.eq_ignore_ascii_case(&dept))
                {
                    continue;
                }

                if let Some(idx) = patient_seen_idx {
                    if let Some(val) = row.get(idx) {
                        if let Some(v) = val.get_int() {
                            total_seen += v;
                        } else if let Some(f) = val.get_float() {
                            total_seen += f as i64;
                        }
                    }
                }
            }
        }

        let hr_str = format!("{:02}:00", fi.dt.hour());
        let date = fi.dt.date();
        *new_data.entry((date, hr_str)).or_insert(0) += total_seen;
    }

    new_data
}
