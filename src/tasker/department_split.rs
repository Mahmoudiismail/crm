use crate::tasker::config::DepartmentSplitConfig;
use anyhow::{Context, Result};
use calamine::{open_workbook, DataType, Reader, Xlsx};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::info;

pub fn run(config: &DepartmentSplitConfig) -> Result<()> {
    info!(
        "Starting department split task for {}",
        config.dashboard_file
    );

    // 1. Build mapping from Chair.xlsx
    let mut mapping: HashMap<String, String> = HashMap::new();
    let chair_path = Path::new(&config.chair_file);
    if !chair_path.exists() {
        anyhow::bail!("Chair mapping file not found at: {:?}", chair_path);
    }

    let mut excel: Xlsx<_> =
        open_workbook(&config.chair_file).context("Failed to open Chair.xlsx")?;
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

    let dashboard_path = PathBuf::from(&config.dashboard_file)
        .canonicalize()
        .context("Failed to canonicalize dashboard_file path")?;

    let dashboard_path_str = {
        let path_str = dashboard_path
            .to_str()
            .context("Failed to convert dashboard_path to string")?;
        path_str.strip_prefix(r"\?\").unwrap_or(path_str)
    };

    let out_dir = PathBuf::from(&config.output_dir);
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
        path_str.strip_prefix(r"\?\").unwrap_or(path_str)
    };

    // Write mapping to a temporary JSON file to pass to PowerShell
    let mapping_json = serde_json::to_string(&mapping)?;
    let tmp_dir = std::env::temp_dir();
    let mapping_file = tmp_dir.join("chair_mapping.json");
    std::fs::write(&mapping_file, mapping_json).context("Failed to write mapping JSON")?;

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

    // Clean up temporary mapping file
    let _ = std::fs::remove_file(&mapping_file);

    info!("Departmental Splitter task completed successfully.");
    Ok(())
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
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
            src.contains("${{target}}: ${{targetPath}}") || src.contains("target: ${{target}}"),
            "Should contain strictly safe string interpolation"
        );
        assert!(
            src.contains("${{target}}"),
            "Should use safe bracket notation for target"
        );
    }
}

#[test]
fn test_special_cells_and_com_cleanup() {
    let src = include_str!("scripts/department_split.ps1");

    // Verify SpecialCells is wrapped properly
    // Use replace to strip out \r on windows for consistent matching
    let src_norm = src.replace("\r\n", "\n");
    assert!(
            src_norm.contains("try {\n                $visibleRows = $dataBodyRange.SpecialCells(12) # xlCellTypeVisible\n            } catch {\n                # This is normal if 0 rows matched the AutoFilter criteria\n                $visibleRows = $null\n            }"),
            "Should contain specific try/catch for SpecialCells"
        );

    // Verify counts are checked
    assert!(
        src.contains("if ($count -gt 0)"),
        "Should verify row count before copying"
    );

    // Verify thorough COM cleanup is performed
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
