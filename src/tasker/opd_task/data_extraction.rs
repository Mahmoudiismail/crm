use crate::tasker::config::OpdAnalysisConfig;
use crate::tasker::opd_task::file_discovery::FileInfo;
use anyhow::{Context, Result};
use calamine::{Data, DataType, Reader};
use chrono::{NaiveDate, Timelike};
use std::collections::HashMap;
use std::path::Path;
use tracing::warn;

/// Filters shared by OPD analysis and the Sat Figures report.
pub struct OpdFilters<'a> {
    pub exclude_specialities: &'a [String],
    pub exclude_emp_names: &'a [String],
    pub exclude_depts: &'a [String],
    pub exclude_speciality_prefixes: &'a [String],
}

impl<'a> From<&'a OpdAnalysisConfig> for OpdFilters<'a> {
    fn from(config: &'a OpdAnalysisConfig) -> Self {
        Self {
            exclude_specialities: &config.exclude_specialities,
            exclude_emp_names: &config.exclude_emp_names,
            exclude_depts: &config.exclude_depts,
            exclude_speciality_prefixes: &config.exclude_speciality_prefixes,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SlotTotals {
    pub slots_booked: i64,
    pub total_slots: i64,
}

#[derive(Debug, Clone, Copy)]
struct FilteredRow {
    total_slot: i64,
    patient_seen: i64,
}

fn numeric_cell(value: Option<&Data>) -> i64 {
    match value {
        Some(v) if !v.is_empty() => v
            .get_int()
            .unwrap_or_else(|| v.get_float().unwrap_or(0.0) as i64),
        _ => 0,
    }
}

fn filtered_rows_from_workbook(path: &Path, filters: &OpdFilters<'_>) -> Result<Vec<FilteredRow>> {
    let mut workbook = calamine::open_workbook_auto(path)
        .with_context(|| format!("Failed to open OPD workbook {:?}", path))?;
    let range = workbook
        .worksheet_range("MIS_Report")
        .with_context(|| format!("Worksheet MIS_Report was not found in {:?}", path))?;
    if range.rows().count() < 5 {
        return Ok(Vec::new());
    }

    let headers: Vec<String> = range
        .rows()
        .nth(4)
        .context("OPD workbook is missing the header row")?
        .iter()
        .map(ToString::to_string)
        .collect();
    let total_slot_idx = headers.iter().position(|h| h == "Total Slot");
    let special_idx = headers.iter().position(|h| h == "Speciality");
    let emp_name_idx = headers.iter().position(|h| h == "Emp Name");
    let dept_idx = headers.iter().position(|h| h == "Dept");
    let patient_seen_idx = headers.iter().position(|h| h == "Total Patient Seen");

    let mut rows = Vec::new();
    for row in range.rows().skip(5) {
        let total_slot = numeric_cell(total_slot_idx.and_then(|idx| row.get(idx)));
        if total_slot == 0 {
            continue;
        }

        let speciality = special_idx
            .and_then(|idx| row.get(idx))
            .unwrap_or(&Data::Empty)
            .to_string();
        let speciality_lower = speciality.to_lowercase();
        if filters
            .exclude_speciality_prefixes
            .iter()
            .any(|prefix| speciality_lower.starts_with(&prefix.to_lowercase()))
        {
            continue;
        }
        if filters
            .exclude_specialities
            .iter()
            .any(|value| value.eq_ignore_ascii_case(&speciality))
        {
            continue;
        }

        let employee = emp_name_idx
            .and_then(|idx| row.get(idx))
            .unwrap_or(&Data::Empty)
            .to_string();
        if filters
            .exclude_emp_names
            .iter()
            .any(|value| value.eq_ignore_ascii_case(&employee))
        {
            continue;
        }

        let department = dept_idx
            .and_then(|idx| row.get(idx))
            .unwrap_or(&Data::Empty)
            .to_string();
        if filters
            .exclude_depts
            .iter()
            .any(|value| value.eq_ignore_ascii_case(&department))
        {
            continue;
        }

        rows.push(FilteredRow {
            total_slot,
            patient_seen: numeric_cell(patient_seen_idx.and_then(|idx| row.get(idx))),
        });
    }
    Ok(rows)
}

/// Aggregate slot totals for a single OPD source workbook using the same filters as OPD Analysis.
pub fn extract_slot_totals(path: &Path, filters: &OpdFilters<'_>) -> Result<SlotTotals> {
    let rows = filtered_rows_from_workbook(path, filters)?;
    if rows.is_empty() {
        anyhow::bail!("No rows remained after applying OPD filters in {:?}", path);
    }
    Ok(rows.into_iter().fold(SlotTotals::default(), |mut totals, row| {
        totals.slots_booked += row.patient_seen;
        totals.total_slots += row.total_slot;
        totals
    }))
}

pub fn extract_new_data(
    config: &OpdAnalysisConfig,
    new_files: Vec<FileInfo>,
) -> HashMap<(NaiveDate, String), i64> {
    let mut new_data: HashMap<(NaiveDate, String), i64> = HashMap::new();
    let filters = OpdFilters::from(config);

    for fi in new_files {
        let total_seen = match filtered_rows_from_workbook(&fi.path, &filters) {
            Ok(rows) => rows.into_iter().map(|row| row.patient_seen).sum(),
            Err(e) => {
                warn!("Failed to process OPD workbook {:?}: {:#}", fi.path, e);
                continue;
            }
        };

        let hr_str = format!("{:02}:00", fi.dt.hour());
        let date = fi.dt.date();
        *new_data.entry((date, hr_str)).or_insert(0) += total_seen;
    }

    new_data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numeric_cell_defaults_empty_or_missing_to_zero() {
        assert_eq!(numeric_cell(None), 0);
        assert_eq!(numeric_cell(Some(&Data::Empty)), 0);
        assert_eq!(numeric_cell(Some(&Data::Int(7))), 7);
        assert_eq!(numeric_cell(Some(&Data::Float(3.9))), 3);
    }

    #[test]
    fn test_opd_filters_share_config_values() {
        let config = OpdAnalysisConfig {
            download_path: String::new(),
            cus_input: String::new(),
            cus_file: String::new(),
            exclude_specialities: vec!["ECG".into()],
            exclude_emp_names: vec!["Example".into()],
            exclude_depts: vec!["Patient Education".into()],
            exclude_speciality_prefixes: vec!["Exe".into()],
            email_to: None,
            email_subject: None,
            special_column_name: "Special".into(),
            date_column_name: "KSA Time".into(),
            check_current_year: false,
        };
        let filters = OpdFilters::from(&config);
        assert_eq!(filters.exclude_specialities[0], "ECG");
        assert_eq!(filters.exclude_emp_names[0], "Example");
        assert_eq!(filters.exclude_depts[0], "Patient Education");
        assert_eq!(filters.exclude_speciality_prefixes[0], "Exe");
    }
}
