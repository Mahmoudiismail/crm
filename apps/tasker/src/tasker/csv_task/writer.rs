use anyhow::Result;
use csv::WriterBuilder;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{info, trace};

pub fn write_processed_records(
    output_file_path: &Path,
    headers: Option<csv::StringRecord>,
    records: &[(String, csv::StringRecord)],
    total_deduped_rows: usize,
    total_filtered_rows: usize,
) -> Result<Option<PathBuf>> {
    info!(
        "Processing ticket files and writing to output: {}",
        output_file_path.display()
    );

    let mut f = File::create(output_file_path)?;
    // Write BOM
    f.write_all(b"\xEF\xBB\xBF")?;

    let mut output_writer = WriterBuilder::new().from_writer(f);

    // Write Headers
    if let Some(h) = headers {
        output_writer.write_record(&h)?;
    }

    info!(
        "Writing {} joined records to output file (deduped: {}, filtered: {}).",
        records.len(),
        total_deduped_rows,
        total_filtered_rows
    );

    for (_, record) in records {
        output_writer.write_record(record)?;
    }

    trace!("Flushing output writer...");
    output_writer.flush()?;

    info!(
        "CSV generation completed successfully. Output written to {}",
        output_file_path.display()
    );

    Ok(Some(output_file_path.to_path_buf()))
}
