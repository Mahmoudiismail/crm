use super::models::CusRow;
use crate::tasker::opd_task::csv_history::parse_ksa_date;
use chrono::{NaiveDate, NaiveTime};
use csv::StringRecord;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct MergeContext {
    pub all_hours: Vec<String>,
    pub other_cols: Vec<String>,
    pub final_rows: Vec<CusRow>,
}

pub fn merge_data(
    cus_records: Vec<StringRecord>,
    cus_headers: &StringRecord,
    hour_columns: Vec<String>,
    new_data: HashMap<(NaiveDate, String), i64>,
) -> MergeContext {
    let mut all_hours_set: HashSet<String> = hour_columns.into_iter().collect();
    for (_d, h) in new_data.keys() {
        all_hours_set.insert(h.clone());
    }
    let mut all_hours: Vec<String> = all_hours_set.into_iter().collect();
    all_hours.sort();

    let mut other_cols = Vec::new();
    for h in cus_headers.iter() {
        if h != "KSA Time" && h != "D" && !all_hours.contains(&h.to_string()) {
            other_cols.push(h.to_string());
        }
    }

    let ksa_idx = cus_headers
        .iter()
        .position(|h| h == "KSA Time")
        .unwrap_or(0);
    let mut current_rows = Vec::new();
    for rec in cus_records {
        if let Some(ksa_str) = rec.get(ksa_idx) {
            if let Some(ksa_date) = parse_ksa_date(ksa_str) {
                let mut row = CusRow {
                    ksa_time: ksa_date,
                    day: "".to_string(),
                    times: HashMap::new(),
                    others: HashMap::new(),
                };
                for (i, h) in cus_headers.iter().enumerate() {
                    let val = rec.get(i).unwrap_or("").to_string();
                    if h == "D" {
                        row.day = val;
                    } else if h == "KSA Time" {
                        // handled
                    } else if let Ok(nt) = NaiveTime::parse_from_str(h, "%H:%M")
                        .or_else(|_| NaiveTime::parse_from_str(h, "%-H:%M"))
                    {
                        let normalized_h = nt.format("%H:00").to_string();
                        if all_hours.contains(&normalized_h) && !val.is_empty() {
                            row.times.insert(normalized_h, val);
                        }
                    } else {
                        row.others.insert(h.to_string(), val);
                    }
                }
                current_rows.push(row);
            }
        }
    }

    // Add new data rows
    // Since new data is aggregated per (Date, Hour), we just create one row per (Date, Hour) initially
    for ((d, h), val) in new_data {
        let mut row = CusRow {
            ksa_time: d,
            day: "".to_string(),
            times: HashMap::new(),
            others: HashMap::new(),
        };
        row.times.insert(h, val.to_string());
        current_rows.push(row);
    }

    // Group by Date and apply merge logic
    let mut grouped: HashMap<NaiveDate, Vec<CusRow>> = HashMap::new();
    for r in current_rows {
        grouped.entry(r.ksa_time).or_default().push(r);
    }

    let mut final_rows = Vec::new();
    for (date, date_rows) in grouped {
        let day_str = date.format("%a").to_string(); // e.g. "Thu"

        // Check for time overlap
        let mut overlap = false;
        for h in &all_hours {
            let count = date_rows.iter().filter(|r| r.times.contains_key(h)).count();
            if count > 1 {
                overlap = true;
                break;
            }
        }

        if overlap {
            // Keep rows separate
            for mut r in date_rows {
                r.day = day_str.clone();
                final_rows.push(r);
            }
        } else {
            // Merge into one row
            let mut merged_times = HashMap::new();
            let mut merged_others = HashMap::new();
            for r in date_rows {
                for (k, v) in r.times {
                    merged_times.insert(k, v);
                }
                for (k, v) in r.others {
                    if !v.is_empty() {
                        merged_others.insert(k, v); // take first non-null/empty essentially
                    }
                }
            }
            final_rows.push(CusRow {
                ksa_time: date,
                day: day_str,
                times: merged_times,
                others: merged_others,
            });
        }
    }

    // Sort final rows by date ascending
    final_rows.sort_by_key(|r| r.ksa_time);

    MergeContext {
        all_hours,
        other_cols,
        final_rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_merge_no_overlap() {
        let mut new_data = HashMap::new();
        new_data.insert(
            (
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                "08:00".to_string(),
            ),
            10,
        );
        new_data.insert(
            (
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                "09:00".to_string(),
            ),
            15,
        );

        let cus_headers = StringRecord::from(vec!["KSA Time", "D"]);
        let cus_records = vec![];

        let ctx = merge_data(cus_records, &cus_headers, vec![], new_data);

        assert_eq!(ctx.final_rows.len(), 1);
        assert_eq!(ctx.final_rows[0].times.get("08:00").unwrap(), "10");
        assert_eq!(ctx.final_rows[0].times.get("09:00").unwrap(), "15");
        assert_eq!(ctx.final_rows[0].day, "Mon"); // 2024-01-01 is Monday
    }
}
