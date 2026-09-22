import sys

path = 'src/tasker/script_manager.rs'
content = open(path, 'r').read()

# Fix fs::rename atomic overrides
content = content.replace(
"""        fs::rename(&tmp_target_path, &target_path).with_context(|| {
            format!(
                "Failed to rename temp script {:?} to {:?}",
                tmp_target_path, target_path
            )
        })?;""",
"""        let _ = fs::remove_file(&target_path);
        fs::rename(&tmp_target_path, &target_path).with_context(|| {
            format!(
                "Failed to rename temp script {:?} to {:?}",
                tmp_target_path, target_path
            )
        })?;"""
)

content = content.replace(
"""        fs::rename(&tmp_metadata_path, &metadata_path).with_context(|| {
            format!("Failed to rename temp metadata file to {:?}", metadata_path)
        })?;""",
"""        let _ = fs::remove_file(&metadata_path);
        fs::rename(&tmp_metadata_path, &metadata_path).with_context(|| {
            format!("Failed to rename temp metadata file to {:?}", metadata_path)
        })?;"""
)

open(path, 'w').write(content)
