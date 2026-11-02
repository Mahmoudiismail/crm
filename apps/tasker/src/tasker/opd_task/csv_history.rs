use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use csv::{ReaderBuilder, StringRecord};
use std::path::Path;

#[derive(Debug)]
pub struct CusHistory {
    pub records: Vec<StringRecord>,
    pub headers: StringRecord,
    pub latest_archived_dt: Option<NaiveDateTime>,
    pub hour_columns: Vec<String>,
}

pub fn parse_ksa_date(date_str: &str) -> Option<NaiveDate> {
    if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(date_str, "%m/%d/%Y") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(date_str, "%d/%m/%Y") {
        return Some(d);
    }
    None
}

pub fn read_history(cus_input_path: &Path) -> Result<CusHistory> {
    let mut cus_records = Vec::new();
    let mut latest_archived_dt: Option<NaiveDateTime> = None;
    let mut hour_columns: Vec<String> = Vec::new();
    let cus_headers;

    if cus_input_path.exists() {
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_path(cus_input_path)?;

        cus_headers = rdr.headers()?.clone();
        for h in cus_headers.iter() {
            if h != "KSA Time"
                && h != "D"
                && (NaiveTime::parse_from_str(h, "%H:%M").is_ok()
                    || NaiveTime::parse_from_str(h, "%-H:%M").is_ok())
            {
                let nt = NaiveTime::parse_from_str(h, "%H:%M")
                    .or_else(|_| NaiveTime::parse_from_str(h, "%-H:%M"))
                    .unwrap();
                hour_columns.push(nt.format("%H:00").to_string());
            }
        }

        for result in rdr.records() {
            let rec = result?;
            cus_records.push(rec.clone());

            // Find KSA Time
            if let Some(ksa_idx) = cus_headers.iter().position(|h| h == "KSA Time") {
                if let Some(ksa_str) = rec.get(ksa_idx) {
                    if let Some(ksa_date) = parse_ksa_date(ksa_str) {
                        for (i, h) in cus_headers.iter().enumerate() {
                            if h != "KSA Time" && h != "D" {
                                if let Ok(nt) = NaiveTime::parse_from_str(h, "%H:%M")
                                    .or_else(|_| NaiveTime::parse_from_str(h, "%-H:%M"))
                                {
                                    if let Some(hr_val) = rec.get(i) {
                                        if !hr_val.is_empty() {
                                            let dt = ksa_date.and_time(nt);
                                            match latest_archived_dt {
                                                None => latest_archived_dt = Some(dt),
                                                Some(ref mut max_dt) => {
                                                    if dt > *max_dt {
                                                        *max_dt = dt;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        // Init default headers if new
        cus_headers = StringRecord::from(vec!["KSA Time", "D"]);
    }

    Ok(CusHistory {
        records: cus_records,
        headers: cus_headers,
        latest_archived_dt,
        hour_columns,
    })
}
