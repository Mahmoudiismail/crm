use crate::tasker::email::message::TicketRow;
use anyhow::Result;
use rust_xlsxwriter::Workbook;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_ARTIFACT_ID: AtomicU64 = AtomicU64::new(0);

/// Build a per-process unique path for a generated email artifact.
pub fn unique_email_artifact_path(bucket_name: &str, suffix: &str) -> PathBuf {
    let safe_name = bucket_name.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let sequence = NEXT_ARTIFACT_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "{safe_name}_{}_{}_{}.{}",
        std::process::id(),
        timestamp,
        sequence,
        suffix.trim_start_matches('.')
    ))
}

#[allow(clippy::too_many_arguments)]
pub fn generate_ticket_attachment(
    bucket_name: &str,
    rows: &[TicketRow],
    headers: &csv::StringRecord,
    is_exception_idx: Option<usize>,
    position_idx: Option<usize>,
    skip_team_idx: Option<usize>,
    month_idx: Option<usize>,
    save_as_csv: bool,
) -> Result<PathBuf> {
    if save_as_csv {
        let csv_path = unique_email_artifact_path(bucket_name, "open_tickets.csv");
        let mut f = std::fs::File::create(&csv_path)?;
        f.write_all(b"\xEF\xBB\xBF")?;
        let mut wtr = csv::WriterBuilder::new().from_writer(f);
        let mut header_rec = vec![];
        for (i, h) in headers.iter().enumerate() {
            if is_exception_idx == Some(i)
                || position_idx == Some(i)
                || skip_team_idx == Some(i)
                || month_idx == Some(i)
            {
                continue;
            }
            header_rec.push(h.to_string());
        }
        wtr.write_record(&header_rec)?;

        for row in rows.iter() {
            if row.status.eq_ignore_ascii_case("closed") {
                continue;
            }
            let mut data_rec = vec![];
            for (c_idx, field) in row.original_row.iter().enumerate() {
                if is_exception_idx == Some(c_idx)
                    || position_idx == Some(c_idx)
                    || skip_team_idx == Some(c_idx)
                    || month_idx == Some(c_idx)
                {
                    continue;
                }
                data_rec.push(field.to_string());
            }
            wtr.write_record(&data_rec)?;
        }
        wtr.flush()?;
        Ok(csv_path)
    } else {
        let xlsx_path = unique_email_artifact_path(bucket_name, "open_tickets.xlsx");
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        let mut out_col_idx = 0;
        for (i, h) in headers.iter().enumerate() {
            if is_exception_idx == Some(i)
                || position_idx == Some(i)
                || skip_team_idx == Some(i)
                || month_idx == Some(i)
            {
                continue;
            }
            worksheet.write_string(0, out_col_idx, h)?;
            out_col_idx += 1;
        }

        let mut write_r_idx = 1;
        for row in rows.iter() {
            if row.status.eq_ignore_ascii_case("closed") {
                continue;
            }
            let mut out_c_idx = 0;
            for (c_idx, field) in row.original_row.iter().enumerate() {
                if is_exception_idx == Some(c_idx)
                    || position_idx == Some(c_idx)
                    || skip_team_idx == Some(c_idx)
                    || month_idx == Some(c_idx)
                {
                    continue;
                }
                worksheet.write_string(write_r_idx as u32, out_c_idx, field)?;
                out_c_idx += 1;
            }
            write_r_idx += 1;
        }
        workbook.save(&xlsx_path)?;
        Ok(xlsx_path)
    }
}

#[cfg(test)]
mod tests {
    use super::unique_email_artifact_path;

    #[test]
    fn generated_artifact_paths_are_unique() {
        let first = unique_email_artifact_path("Call Center", "email.html");
        let second = unique_email_artifact_path("Call Center", "email.html");
        assert_ne!(first, second);
        assert!(first
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with("email.html")));
        assert!(second
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with("email.html")));
    }
}
