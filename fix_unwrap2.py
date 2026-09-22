import sys

content = open('src/tasker/department_split.rs', 'r').read()

# Fix dashboard_path unwrap
old_dash = """    let dashboard_path_str = dashboard_path
        .to_str()
        .unwrap()
        .strip_prefix(r"\\?\\")
        .unwrap_or(dashboard_path.to_str().unwrap());"""
new_dash = """    let dashboard_path_str_full = dashboard_path.to_str().ok_or_else(|| anyhow::anyhow!("Failed to convert dashboard path to string: {:?}", dashboard_path))?;
    let dashboard_path_str = dashboard_path_str_full.strip_prefix(r"\\\\?\\").unwrap_or(dashboard_path_str_full);"""

content = content.replace(old_dash, new_dash)

# Fix out_dir unwrap
old_out = """    let out_dir_str = out_dir_canon
        .to_str()
        .unwrap()
        .strip_prefix(r"\\?\\")
        .unwrap_or(out_dir_canon.to_str().unwrap());"""
new_out = """    let out_dir_str_full = out_dir_canon.to_str().ok_or_else(|| anyhow::anyhow!("Failed to convert output directory path to string: {:?}", out_dir_canon))?;
    let out_dir_str = out_dir_str_full.strip_prefix(r"\\\\?\\").unwrap_or(out_dir_str_full);"""

content = content.replace(old_out, new_out)

# Fix mapping_file_str unwrap
old_map = """    let mapping_file_str = mapping_file.to_str().unwrap();"""
new_map = """    let mapping_file_str = mapping_file.to_str().ok_or_else(|| anyhow::anyhow!("Failed to convert mapping file path to string: {:?}", mapping_file))?;"""

content = content.replace(old_map, new_map)

open('src/tasker/department_split.rs', 'w').write(content)
