use crate::tasker::config::{OpdAnalysisConfig, SatFiguresConfig};
use anyhow::{Context, Result};
use csv::WriterBuilder;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use tracing::info;

fn resolve(path: &str) -> PathBuf {
    crate::tasker::csv_task::resolve_relative_to_exe_dir(path)
}

fn is_source_report(path: &std::path::Path) -> bool {
    let name = path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or_default();
    let extension = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default();
    name.to_ascii_lowercase()
        .starts_with("opd- average patients seen no of schedule_")
        && !name.starts_with("~$")
        && ["xlsx", "xlsm", "xlsb", "xls"]
            .iter()
            .any(|supported| extension.eq_ignore_ascii_case(supported))
}

fn latest_source_report(download_dir: &std::path::Path) -> Result<PathBuf> {
    let mut latest: Option<(SystemTime, PathBuf)> = None;

    for entry in walkdir::WalkDir::new(download_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path().to_path_buf();
        if !is_source_report(&path) {
            continue;
        }
        let metadata = entry
            .metadata()
            .with_context(|| format!("Failed to read metadata for {:?}", path))?;
        let created = metadata
            .created()
            .or_else(|_| metadata.modified())
            .with_context(|| format!("Could not determine creation time for {:?}", path))?;
        if latest
            .as_ref()
            .map_or(true, |(timestamp, _)| created > *timestamp)
        {
            latest = Some((created, path));
        }
    }

    latest
        .map(|(_, path)| path)
        .context("No OPD Average Patients Seen workbook matching the Sat Figures prefix was found")
}

fn get_email_script() -> &'static str {
    r#"
param(
    [string]$CsvPath,
    [string]$EmailTo,
    [string]$EmailCc,
    [string]$EmailSubject
)
$ErrorActionPreference = "Stop"
$imagePath = $null
$bitmap = $null
$graphics = $null
$headerFont = $null
$valueFont = $null
$brush = $null
$headerBrush = $null
$borderPen = $null
$outlook = $null
$mail = $null
$attachment = $null
try {
    Add-Type -AssemblyName System.Drawing
    $row = @(Import-Csv -LiteralPath $CsvPath)
    if ($row.Count -ne 1) { throw "Sat Figures CSV must contain exactly one summary row." }

    $columns = @("Slots Booked", "Total Slots", "Vacant Slots", "%")
    $widths = @(240, 240, 240, 240)
    $width = ($widths | Measure-Object -Sum).Sum
    $height = 118
    $bitmap = [System.Drawing.Bitmap]::new($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.Clear([System.Drawing.Color]::White)
    $headerFont = [System.Drawing.Font]::new("Segoe UI", 12, [System.Drawing.FontStyle]::Bold)
    $valueFont = [System.Drawing.Font]::new("Segoe UI", 18, [System.Drawing.FontStyle]::Regular)
    $brush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(35, 45, 55))
    $headerBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(46, 92, 135))
    $whiteBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
    $borderPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(190, 200, 210), 1)

    $x = 0
    for ($i = 0; $i -lt $columns.Count; $i++) {
        $cellWidth = $widths[$i]
        $graphics.FillRectangle($headerBrush, $x, 0, $cellWidth, 48)
        $graphics.DrawRectangle($borderPen, $x, 0, $cellWidth, $height - 1)
        $headerRect = [System.Drawing.RectangleF]::new($x + 6, 0, $cellWidth - 12, 48)
        $valueRect = [System.Drawing.RectangleF]::new($x + 6, 49, $cellWidth - 12, 68)
        $format = [System.Drawing.StringFormat]::new()
        $format.Alignment = [System.Drawing.StringAlignment]::Center
        $format.LineAlignment = [System.Drawing.StringAlignment]::Center
        $graphics.DrawString($columns[$i], $headerFont, $whiteBrush, $headerRect, $format)

        $value = [string]$row[0].PSObject.Properties.Item($columns[$i]).Value
        if ($columns[$i] -eq "%") {
            $number = 0.0
            if (-not [double]::TryParse($value, [Globalization.NumberStyles]::Any, [Globalization.CultureInfo]::InvariantCulture, [ref]$number)) {
                throw "Invalid percentage value in Sat Figures CSV: $value"
            }
            $value = $number.ToString("P2", [Globalization.CultureInfo]::InvariantCulture)
        }
        $graphics.DrawString($value, $valueFont, $brush, $valueRect, $format)
        $format.Dispose()
        $x += $cellWidth
    }

    $imagePath = Join-Path $env:TEMP ("sat_figures_" + [guid]::NewGuid().ToString("N") + ".png")
    $bitmap.Save($imagePath, [System.Drawing.Imaging.ImageFormat]::Png)

    $outlook = New-Object -ComObject Outlook.Application
    $mail = $outlook.CreateItem(0)
    $mail.To = $EmailTo
    if ($EmailCc) { $mail.CC = $EmailCc }
    $mail.Subject = $EmailSubject
    $attachment = $mail.Attachments.Add($imagePath)
    $attachment.PropertyAccessor.SetProperty(
        "http://schemas.microsoft.com/mapi/proptag/0x3712001F",
        "SatFiguresImage"
    )
    $mail.HTMLBody = "<html><body><img src='cid:SatFiguresImage' alt='Sat Figures report' /></body></html>"
    $mail.Send()
    Write-Output "TRACE: Sat Figures inline image email sent successfully."
} catch {
    Write-Error "Sat Figures email generation failed: $_"
    exit 1
} finally {
    if ($graphics) { $graphics.Dispose() }
    if ($bitmap) { $bitmap.Dispose() }
    if ($headerFont) { $headerFont.Dispose() }
    if ($valueFont) { $valueFont.Dispose() }
    if ($brush) { $brush.Dispose() }
    if ($headerBrush) { $headerBrush.Dispose() }
    if ($whiteBrush) { $whiteBrush.Dispose() }
    if ($borderPen) { $borderPen.Dispose() }
    if ($attachment) { try { [System.Runtime.Interopservices.Marshal]::ReleaseComObject($attachment) | Out-Null } catch {} }
    if ($mail) { try { [System.Runtime.Interopservices.Marshal]::ReleaseComObject($mail) | Out-Null } catch {} }
    if ($outlook) {
        try { [System.Runtime.Interopservices.Marshal]::ReleaseComObject($outlook) | Out-Null } catch {}
    }
    if ($imagePath -and (Test-Path -LiteralPath $imagePath)) { Remove-Item -LiteralPath $imagePath -Force -ErrorAction SilentlyContinue }
}
"#
}

pub fn run(config: &SatFiguresConfig, opd_config: &OpdAnalysisConfig) -> Result<()> {
    let download_dir = resolve(&config.download_path);
    if !download_dir.is_dir() {
        anyhow::bail!(
            "Sat Figures download directory does not exist: {:?}",
            download_dir
        );
    }

    // Match the workbook query: select the newest-created file with the exact report prefix.
    let latest = latest_source_report(&download_dir)?;
    let filters = crate::tasker::opd_task::data_extraction::OpdFilters::from(opd_config);
    let totals = crate::tasker::opd_task::data_extraction::extract_slot_totals(&latest, &filters)
        .with_context(|| format!("Failed to calculate Sat Figures from {:?}", latest))?;

    if totals.total_slots == 0 {
        anyhow::bail!(
            "Sat Figures total slot count is zero; cannot calculate occupancy percentage"
        );
    }

    let vacant_slots = totals.total_slots - totals.slots_booked;
    let percentage = totals.slots_booked as f64 / totals.total_slots as f64;
    let output_path = resolve(&config.output_file);
    if let Some(parent) = output_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create Sat Figures output directory {:?}", parent)
        })?;
    }

    let mut writer = WriterBuilder::new()
        .from_path(&output_path)
        .with_context(|| format!("Failed to create Sat Figures CSV {:?}", output_path))?;
    writer.write_record(["Slots Booked", "Total Slots", "Vacant Slots", "%"])?;
    writer.write_record([
        totals.slots_booked.to_string(),
        totals.total_slots.to_string(),
        vacant_slots.to_string(),
        format!("{percentage:.10}"),
    ])?;
    writer.flush()?;

    let csv_path = output_path.to_string_lossy().into_owned();
    let manager = crate::tasker::script_manager::ScriptManager::new();
    manager.run_script_with_args(
        "Sat Figures",
        "sat_figures_email.ps1",
        get_email_script(),
        &[
            ("-CsvPath", &csv_path),
            ("-EmailTo", &config.email_to),
            ("-EmailCc", &config.email_cc),
            ("-EmailSubject", &config.email_subject),
        ],
    )?;

    info!(
        "Sat Figures completed from {:?}: booked={}, total={}, vacant={}, percentage={:.2}%",
        latest,
        totals.slots_booked,
        totals.total_slots,
        vacant_slots,
        percentage * 100.0
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_report_filter_matches_only_expected_prefix_and_extensions() {
        assert!(is_source_report(std::path::Path::new(
            "OPD- Average Patients seen No of Schedule_01.xlsx"
        )));
        assert!(is_source_report(std::path::Path::new(
            "opd- average patients seen no of schedule_02.xlsm"
        )));
        assert!(!is_source_report(std::path::Path::new(
            "~$OPD- Average Patients seen No of Schedule_01.xlsx"
        )));
        assert!(!is_source_report(std::path::Path::new("Other report.xlsx")));
        assert!(!is_source_report(std::path::Path::new(
            "OPD- Average Patients seen No of Schedule_01.csv"
        )));
    }

    #[test]
    fn email_script_uses_configured_recipients_and_cleans_up_temporary_files() {
        let script = get_email_script();
        assert!(script.contains("$mail.To = $EmailTo"));
        assert!(script.contains("cid:SatFiguresImage"));
        assert!(script.contains("$mail.Send()"));
        assert!(script.contains("sat_figures_"));
        assert!(script.contains("Remove-Item -LiteralPath $imagePath"));
    }

    #[test]
    fn email_script_formats_percentage_as_percent() {
        assert!(get_email_script().contains("$number.ToString(\"P2\""));
    }
}
