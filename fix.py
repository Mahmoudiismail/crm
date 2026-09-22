import sys

content = open('src/tasker/department_split.rs', 'r').read()
content = content.replace('let dashboard_path_str_full = dashboard_path.to_str().ok_or_else(|| anyhow::anyhow!("Failed to convert dashboard path to string: {:?}", dashboard_path))?;\n    let dashboard_path_str_full = dashboard_path.to_str().ok_or_else(|| anyhow::anyhow!("Failed to convert dashboard path to string: {:?}", dashboard_path))?;\n    let dashboard_path_str = dashboard_path_str_full_str_full\n        .to_str()\n        .unwrap()\n        .strip_prefix(r"\\\\?\\")\n        .unwrap_or(dashboard_path.to_str().unwrap());',
"""let dashboard_path_str_full = dashboard_path.to_str().ok_or_else(|| anyhow::anyhow!("Failed to convert dashboard path to string: {:?}", dashboard_path))?;
    let dashboard_path_str = dashboard_path_str_full.strip_prefix(r"\\\\?\\").unwrap_or(dashboard_path_str_full);""")

content = content.replace('let out_dir_str_full = out_dir_canon.to_str().ok_or_else(|| anyhow::anyhow!("Failed to convert output directory path to string: {:?}", out_dir_canon))?;\n    let out_dir_str = out_dir_str_full\n        .to_str()\n        .unwrap()\n        .strip_prefix(r"\\\\?\\")\n        .unwrap_or(out_dir_canon.to_str().unwrap());',
"""let out_dir_str_full = out_dir_canon.to_str().ok_or_else(|| anyhow::anyhow!("Failed to convert output directory path to string: {:?}", out_dir_canon))?;
    let out_dir_str = out_dir_str_full.strip_prefix(r"\\\\?\\").unwrap_or(out_dir_str_full);""")

open('src/tasker/department_split.rs', 'w').write(content)
