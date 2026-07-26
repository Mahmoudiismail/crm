use crate::tasker::email::message::TicketRow;
use anyhow::Result;
use rust_xlsxwriter::Workbook;
use std::io::Write;
use std::path::PathBuf;

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
    let tmp_dir = std::env::temp_dir();
    let safe_name = bucket_name.replace(|c: char| !c.is_ascii_alphanumeric(), "_");

    if save_as_csv {
        let csv_path = tmp_dir.join(format!("{}_open_tickets.csv", safe_name));
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
        let xlsx_path = tmp_dir.join(format!("{}_open_tickets.xlsx", safe_name));
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
