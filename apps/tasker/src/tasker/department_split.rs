use crate::tasker::config::DepartmentSplitConfig;
use anyhow::{Context, Result};
use calamine::{open_workbook, DataType, Reader, Xlsx};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use tracing::info;

fn normalize_windows_path_for_excel(path: &str) -> &str {
    path.strip_prefix(r"\\?\").unwrap_or(path)
}

fn write_mapping_temp_file(mapping: &HashMap<String, String>) -> Result<PathBuf> {
    let mapping_json = serde_json::to_vec(mapping)?;
    let mut file = tempfile::Builder::new()
        .prefix("chair_mapping_")
        .suffix(".json")
        .tempfile()
        .context("Failed to create temporary mapping JSON")?;
    file.write_all(&mapping_json)
        .context("Failed to write mapping JSON")?;
    let (file, path) = file
        .keep()
        .map_err(|error| anyhow::anyhow!("Failed to preserve mapping JSON: {}", error.error))?;
    drop(file);
    Ok(path)
}

pub fn run(config: &DepartmentSplitConfig) -> Result<()> {
    info!(
        "Starting department split task for {}",
        config.dashboard_file
    );

    // 1. Build mapping from Chair.xlsx
    let mut mapping: HashMap<String, String> = HashMap::new();
    let chair_path = crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.chair_file);
    if !chair_path.exists() {
        anyhow::bail!("Chair mapping file not found at: {:?}", chair_path);
    }

    let mut excel: Xlsx<_> = open_workbook(&chair_path).context("Failed to open Chair.xlsx")?;
    let sheet_names = excel.sheet_names().to_owned();
    if sheet_names.is_empty() {
        anyhow::bail!("No sheets found in Chair.xlsx");
    }

    if let Ok(range) = excel.worksheet_range(&sheet_names[0]) {
        let mut is_header = true;
        for row in range.rows() {
            if is_header {
                is_header = false;
                continue;
            }
            if row.len() >= 2 {
                if let (Some(dep), Some(chir)) = (row[0].as_string(), row[1].as_string()) {
                    let clean_dep = dep.trim().to_uppercase();
                    let clean_chir = chir.trim().to_uppercase();
                    if !clean_dep.is_empty() && !clean_chir.is_empty() {
                        mapping.insert(clean_dep, clean_chir);
                    }
                }
            }
        }
    } else {
        anyhow::bail!("Failed to read data from the first sheet in Chair.xlsx");
    }

    info!("Loaded {} department mappings.", mapping.len());

    let dashboard_path =
        crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.dashboard_file)
            .canonicalize()
            .context("Failed to canonicalize dashboard_file path")?;

    let dashboard_path_str = {
        let path_str = dashboard_path
            .to_str()
            .context("Failed to convert dashboard_path to string")?;
        normalize_windows_path_for_excel(path_str)
    };

    let out_dir = crate::tasker::csv_task::resolve_relative_to_exe_dir(&config.output_dir);
    if !out_dir.exists() {
        std::fs::create_dir_all(&out_dir).context("Failed to create output directory")?;
    }

    let out_dir_canon = out_dir
        .canonicalize()
        .context("Failed to canonicalize output_dir")?;
    let out_dir_str = {
        let path_str = out_dir_canon
            .to_str()
            .context("Failed to convert out_dir_canon to string")?;
        normalize_windows_path_for_excel(path_str)
    };

    // Use a unique path so concurrent task runs cannot overwrite each other's mapping.
    let mapping_file = write_mapping_temp_file(&mapping)?;
    let _mapping_cleanup_guard = crate::utils::FileCleanupGuard::new(&mapping_file);

    let mapping_file_str = mapping_file
        .to_str()
        .context("Failed to convert mapping_file to string")?;

    let ps_script = include_str!("scripts/department_split.ps1");

    crate::tasker::script_manager::ScriptManager::new().run_script_with_args(
        "Department Split",
        "department_split.ps1",
        ps_script,
        &[
            ("-DashboardPath", dashboard_path_str),
            ("-OutputDir", out_dir_str),
            ("-MappingFile", mapping_file_str),
        ],
    )?;

    info!("Departmental Splitter task completed successfully.");
    Ok(())
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    #[test]
    fn excel_path_normalization_removes_windows_extended_prefix() {
        assert_eq!(
            super::normalize_windows_path_for_excel(r"\\?\C:\Reports\10-OP.xlsm"),
            r"C:\Reports\10-OP.xlsm"
        );
        assert_eq!(
            super::normalize_windows_path_for_excel(r"C:\Reports\10-OP.xlsm"),
            r"C:\Reports\10-OP.xlsm"
        );
    }

    #[test]
    fn mapping_temp_files_are_unique_and_contain_json() {
        use super::write_mapping_temp_file;
        use std::collections::HashMap;

        let mapping = HashMap::from([("CARDIOLOGY".to_string(), "CHAIR A".to_string())]);
        let first = write_mapping_temp_file(&mapping).unwrap();
        let second = write_mapping_temp_file(&mapping).unwrap();
        let _first_guard = crate::utils::FileCleanupGuard::new(&first);
        let _second_guard = crate::utils::FileCleanupGuard::new(&second);

        assert_ne!(first, second);
        let parsed: HashMap<String, String> =
            serde_json::from_slice(&std::fs::read(&first).unwrap()).unwrap();
        assert_eq!(parsed, mapping);
    }

    #[test]
    fn test_department_header_scan_reads_range_in_bulk() {
        let src = include_str!("scripts/department_split.ps1");
        assert!(src.contains("$headerValues = $Sheet.Range($Sheet.Cells.Item(1, 1), $Sheet.Cells.Item(20, 50)).Value2"));
        assert!(src.contains("$cellVal = [string]$headerValues[$r, $c]"));
    }

    #[test]
    fn test_powershell_interpolation_safety() {
        let src = include_str!("scripts/department_split.ps1");

        // Assert the problematic unsafe interpolation does not exist
        assert!(
            !src.contains(&format!("{}{}", "$target", ": $targetPath")),
            "Should not contain unsafe interpolation"
        );
        assert!(
            !src.contains(&format!("{}{}", "$target", ":")),
            "Should not contain ambiguous interpolation"
        );

        // Assert the safe version does exist
        assert!(
            src.contains("${target}.xlsm") && src.contains("(target: ${target})"),
            "Should use braced interpolation where a variable touches adjacent text"
        );
        assert!(
            src.contains("${target}"),
            "Should use safe braced interpolation for target"
        );
    }
}

#[test]
fn test_department_split_uses_single_source_read_and_bulk_transfer() {
    let src = include_str!("scripts/department_split.ps1");

    assert!(
        src.contains("$sourceValues = $sourceRange.Value2"),
        "Should read the OPD data range once"
    );
    assert!(
        src.contains("$targetRowsMap[$targetChair].Add($r)"),
        "Should group source row indexes by target"
    );
    assert!(
        src.contains("$destinationRange.Value2 = $outputValues"),
        "Should write each target data matrix in one COM operation"
    );
    assert!(
        !src.contains(".AutoFilter(") && !src.contains(".SpecialCells(12)"),
        "Should not filter and transfer source rows separately for each target"
    );

    // Verify thorough COM cleanup is performed.
    assert!(
        src.contains("[System.Runtime.InteropServices.Marshal]::ReleaseComObject($TargetWB)"),
        "Should release TargetWB"
    );
    assert!(
        src.contains("[System.Runtime.InteropServices.Marshal]::ReleaseComObject($Workbook)"),
        "Should release Master Workbook"
    );
    assert!(
        src.contains("$Excel = $null"),
        "Should set Excel to null for GC"
    );
}
