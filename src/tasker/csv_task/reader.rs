use anyhow::Result;
use csv::{Reader, StringRecord};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tracing::{error, warn};

#[derive(Debug)]
pub struct TicketCsvReader {
    pub reader: Reader<BufReader<File>>,
    pub headers: csv::StringRecord,
    pub file_path: std::path::PathBuf,
}

impl TicketCsvReader {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Option<Self>> {
        let path = file_path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let mut rdr = crate::utils::build_csv_reader_from_reader(BufReader::new(file));

        let headers = match rdr.headers() {
            Ok(h) => {
                if h.is_empty() {
                    warn!("Empty file: {}", path.display());
                    return Ok(None);
                }
                h.clone()
            }
            Err(e) => {
                warn!("Empty or invalid file: {} ({})", path.display(), e);
                return Ok(None);
            }
        };

        Ok(Some(Self {
            reader: rdr,
            headers,
            file_path: path,
        }))
    }

    pub fn read_record(&mut self) -> Result<Option<StringRecord>> {
        let mut record = StringRecord::new();
        match self.reader.read_record(&mut record) {
            Ok(true) => Ok(Some(record)),
            Ok(false) => Ok(None),
            Err(e) => {
                let line_num = e.position().map(|p| p.line()).unwrap_or(0) as usize;
                let diagnostic_info = crate::utils::generate_csv_diagnostic_context_from_file(
                    &self.file_path,
                    line_num,
                );

                error!(
                    "CSV parsing error in file {:?} at line {}: {}\nDiagnostic Context (±20 lines):\n{}",
                    self.file_path, line_num, e, diagnostic_info
                );
                anyhow::bail!("Failed to parse ticket report CSV: {}", e);
            }
        }
    }

    pub fn records(&mut self) -> impl Iterator<Item = Result<StringRecord>> + '_ {
        std::iter::from_fn(move || self.read_record().transpose())
    }
}
