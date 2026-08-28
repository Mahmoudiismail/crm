use super::merging::MergeContext;
use anyhow::Result;
use csv::WriterBuilder;
use std::path::Path;
use tracing::info;

pub fn export_csv(cus_file_path: &Path, ctx: MergeContext) -> Result<()> {
    let mut wtr = WriterBuilder::new().from_path(cus_file_path)?;

    // headers: KSA Time | time columns | D | other columns
    let mut final_headers = vec!["KSA Time".to_string()];
    final_headers.extend(ctx.all_hours.clone());
    final_headers.push("D".to_string());
    final_headers.extend(ctx.other_cols.clone());

    wtr.write_record(&final_headers)?;

    for r in ctx.final_rows {
        let mut rec = Vec::new();
        rec.push(r.ksa_time.format("%Y-%m-%d").to_string());
        for h in &ctx.all_hours {
            rec.push(r.times.get(h).cloned().unwrap_or_default());
        }
        rec.push(r.day);
        for h in &ctx.other_cols {
            rec.push(r.others.get(h).cloned().unwrap_or_default());
        }
        wtr.write_record(&rec)?;
    }

    wtr.flush()?;
    info!("OpdAnalysis CSV export completed successfully.");
    Ok(())
}
