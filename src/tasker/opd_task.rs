use crate::tasker::config::OpdAnalysisConfig;
use anyhow::Result;
use calamine::{Data, DataType, Reader};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use std::collections::{HashMap, HashSet};
use tracing::{info, warn};
use walkdir::WalkDir;

pub fn run(config: &OpdAnalysisConfig) -> Result<()> {
    let download_dir_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.download_path);
    let cus_input_path = crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.cus_input);
    let cus_file_path = crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.cus_file);

    info!("Running OpdAnalysis for folder: {:?}", download_dir_path);

    // 1. Read existing CUS and find latest hour
    let mut cus_records = Vec::new();
    let cus_headers;
    let mut latest_archived_dt: Option<NaiveDateTime> = None;
    let mut hour_columns: Vec<String> = Vec::new();

    // Helper to parse dates in multiple formats
    let parse_ksa_date = |date_str: &str| -> Option<NaiveDate> {
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
    };

    if cus_input_path.exists() {
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_path(&cus_input_path)?;

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

    let process_from = latest_archived_dt.map(|dt| dt + chrono::Duration::hours(1));
    info!(
        "Latest archived hour: {:?}. Processing from: {:?}",
        latest_archived_dt, process_from
    );

    // 2. Scan and filter files
    struct FileInfo {
        path: std::path::PathBuf,
        dt: NaiveDateTime,
    }
    let mut new_files = Vec::new();

    for entry in WalkDir::new(&download_dir_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let fname = entry.file_name().to_string_lossy().to_string();
        if fname.starts_with("~$") {
            continue;
        }
        if !fname.to_lowercase().contains("average patients seen") {
            continue;
        }
        let ext = entry
            .path()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !["xlsx", "xlsm", "xlsb", "xls"].contains(&ext.as_str()) {
            continue;
        }

        // Extract date time from name: DD-MM-YYYY_HHMMSS
        // Strip extension safely
        let base = if let Some(idx) = fname.rfind('.') {
            &fname[..idx]
        } else {
            &fname
        };

        let parts: Vec<&str> = base.split('_').collect();
        if parts.len() >= 2 {
            let date_str = parts[parts.len() - 2].trim();
            let time_str = parts[parts.len() - 1]
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>();
            if time_str.len() == 6 {
                if let Ok(d) = NaiveDate::parse_from_str(date_str, "%d-%m-%Y") {
                    let hr: u32 = time_str[0..2].parse().unwrap_or(0);
                    let min: u32 = time_str[2..4].parse().unwrap_or(0);
                    let sec: u32 = time_str[4..6].parse().unwrap_or(0);
                    if let Some(t) = NaiveTime::from_hms_opt(hr, min, sec) {
                        let dt = d.and_time(t);
                        if let Some(pf) = process_from {
                            if dt >= pf {
                                new_files.push(FileInfo {
                                    path: entry.path().to_path_buf(),
                                    dt,
                                });
                            }
                        } else {
                            new_files.push(FileInfo {
                                path: entry.path().to_path_buf(),
                                dt,
                            });
                        }
                    }
                }
            }
        }
    }

    new_files.sort_by_key(|f| f.dt);
    info!("Found {} new files to process", new_files.len());

    // 3. Process new files
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

    if new_data.is_empty() {
        info!("No new data to append.");
        return Ok(());
    }

    // 4. Merge Data
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

    // Matrix: Date -> (RowId -> {ColName -> String})
    // PowerQuery merge logic:
    // We group rows by KSA Time.
    // If populated time columns do NOT overlap across multiple rows for a date, we merge them into a single row.
    // Otherwise, we keep them as separate rows.

    // Convert existing CUS to a struct
    #[derive(Clone)]
    struct CusRow {
        ksa_time: NaiveDate,
        day: String,
        times: HashMap<String, String>,
        others: HashMap<String, String>,
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

    // 5. Write out
    let mut wtr = WriterBuilder::new().from_path(&cus_file_path)?;

    // headers: KSA Time | time columns | D | other columns
    let mut final_headers = vec!["KSA Time".to_string()];
    final_headers.extend(all_hours.clone());
    final_headers.push("D".to_string());
    final_headers.extend(other_cols.clone());

    wtr.write_record(&final_headers)?;

    for r in final_rows {
        let mut rec = Vec::new();
        rec.push(r.ksa_time.format("%Y-%m-%d").to_string());
        for h in &all_hours {
            rec.push(r.times.get(h).cloned().unwrap_or_default());
        }
        rec.push(r.day);
        for h in &other_cols {
            rec.push(r.others.get(h).cloned().unwrap_or_default());
        }
        wtr.write_record(&rec)?;
    }

    wtr.flush()?;
    info!("OpdAnalysis completed successfully.");

    // 6. Generate Image and Email using PowerShell
    if let (Some(email_to), Some(email_subject)) = (&config.email_to, &config.email_subject) {
        let ps_script = format!(
            r#"
$ErrorActionPreference = "Stop"

$csvPath = "{0}"
$emailTo = "{1}"
$emailSubject = "{2}"
$specialCol = "{3}"
$dateCol = "{4}"
$checkCurrentYear = {5}

$excel = New-Object -ComObject Excel.Application
$excel.Visible = $false
$excel.DisplayAlerts = $false
$excel.ScreenUpdating = $false
$excel.EnableEvents = $false

try {{
    $workbook = $excel.Workbooks.Open($csvPath)
    $ws = $workbook.Sheets.Item(1)

    # Apply AutoFilter
    if ($ws.AutoFilterMode) {{
        $ws.AutoFilterMode = $false
    }}

    $usedRange = $ws.UsedRange
    $headers = $usedRange.Rows(1).Value2

    if (-not $headers) {{ throw "No headers found in CSV." }}

    # Convert 2D array to 1D
    $headerArray = @()
    for ($i = 1; $i -le $headers.GetLength(1); $i++) {{
        $headerArray += $headers[1, $i]
    }}

    $dColIdx = [array]::IndexOf($headerArray, "D") + 1
    $specialColIdx = [array]::IndexOf($headerArray, $specialCol) + 1
    $dateColIdx = [array]::IndexOf($headerArray, $dateCol) + 1

    $dayName = (Get-Date).ToString("ddd") # "Mon", "Tue"

    if ($dColIdx -gt 0) {{
        $usedRange.AutoFilter($dColIdx, $dayName) | Out-Null
    }}

    if ($specialColIdx -gt 0) {{
        $usedRange.AutoFilter($specialColIdx, "=") | Out-Null
    }}

    if ($checkCurrentYear -and $dateColIdx -gt 0) {{
        $yearStart = Get-Date -Year (Get-Date).Year -Month 1 -Day 1 -Hour 0 -Minute 0 -Second 0
        $yearEnd = $yearStart.AddYears(1)
        # Dates in Excel are often represented as numbers or strings depending on CSV load
        # For CSV loaded into Excel, standard > < text filter works if dates are formatted yyyy-mm-dd
        $usedRange.AutoFilter($dateColIdx, ">=$($yearStart.ToString('yyyy-MM-dd'))", 1, "<$($yearEnd.ToString('yyyy-MM-dd'))") | Out-Null
    }}

    $visibleRows = $usedRange.SpecialCells(12) # xlCellTypeVisible

    # Find last visible row
    $lastRow = 1
    foreach ($area in $visibleRows.Areas) {{
        $areaLastRow = $area.Row + $area.Rows.Count - 1
        if ($areaLastRow -gt $lastRow) {{
            $lastRow = $areaLastRow
        }}
    }}

    # Hide blank columns at last row
    for ($c = 1; $c -le $usedRange.Columns.Count; $c++) {{
        $val = $ws.Cells.Item($lastRow, $c).Text
        if ([string]::IsNullOrWhiteSpace($val)) {{
            $ws.Columns.Item($c).Hidden = $true
        }}
    }}

    # Hide D column
    if ($dColIdx -gt 0) {{
        $ws.Columns.Item($dColIdx).Hidden = $true
    }}

    $imagePath = Join-Path $env:TEMP "Query1.jpg"
    if (Test-Path $imagePath) {{ Remove-Item $imagePath -Force }}

    $usedRange.CopyPicture(1, 2) | Out-Null # xlScreen=1, xlPicture=2

    Start-Sleep -Seconds 1

    # Find exact width and height of visible range to size the chart appropriately
    $totalWidth = 0
    $totalHeight = 0
    for ($c = 1; $c -le $usedRange.Columns.Count; $c++) {{
        if (-not $ws.Columns.Item($c).Hidden) {{
            $totalWidth += $ws.Columns.Item($c).Width
        }}
    }}
    for ($r = 1; $r -le $lastRow; $r++) {{
        if (-not $ws.Rows.Item($r).Hidden) {{
            $totalHeight += $ws.Rows.Item($r).Height
        }}
    }}

    if ($totalWidth -lt 50) {{ $totalWidth = 50 }}
    if ($totalHeight -lt 50) {{ $totalHeight = 50 }}

    $chartObj = $ws.ChartObjects().Add(10, 10, $totalWidth, $totalHeight)
    $chartObj.Activate()
    $chartObj.Chart.Paste() | Out-Null
    Start-Sleep -Seconds 1

    $chartObj.Chart.Export($imagePath, "JPG") | Out-Null
    $chartObj.Delete()

    if (Test-Path $imagePath) {{
        $outlook = New-Object -ComObject Outlook.Application
        $mail = $outlook.CreateItem(0)
        $mail.To = $emailTo
        $mail.Subject = $emailSubject
        $mail.Attachments.Add($imagePath) | Out-Null
        $mail.Send()
        Write-Output "Email sent successfully."
    }} else {{
        Write-Error "Failed to generate image from filtered table."
    }}

}} finally {{
    if ($workbook) {{ $workbook.Close($false) }}
    $excel.Quit()
    [System.Runtime.Interopservices.Marshal]::ReleaseComObject($excel) | Out-Null
}}
"#,
            cus_file_path
                .canonicalize()?
                .to_string_lossy()
                .replace(r"\\?\", ""),
            email_to,
            email_subject,
            config.special_column_name,
            config.date_column_name,
            if config.check_current_year {
                "$true"
            } else {
                "$false"
            }
        );

        let mut temp_file = tempfile::Builder::new()
            .prefix("opd_analysis_email_")
            .suffix(".ps1")
            .tempfile()?;

        use std::io::Write;
        temp_file.write_all(ps_script.as_bytes())?;
        temp_file.as_file().sync_all()?;

        let (file, path) = temp_file.keep()?;
        drop(file);

        info!("Running PowerShell for generating and emailing image...");
        let output = std::process::Command::new("powershell")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-File")
            .arg(&path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("PowerShell script failed:\n{}", stderr);
            anyhow::bail!("Failed to generate/email image. PS Error: {}", stderr);
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            info!("PowerShell email success: {}", stdout);
        }

        let _ = std::fs::remove_file(path);
    }

    Ok(())
}
