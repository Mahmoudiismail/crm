with open('src/crm/fetcher.rs', 'r') as f:
    content = f.read()

# We need to move has_recent_download BEFORE `mod tests`
# We'll extract `fn has_recent_download` up to its end.

# The function is currently at the end of the file.
fn_code = """
fn has_recent_download(download_dir: &Path, prefix: &str) -> bool {
    let threshold = SystemTime::now() - std::time::Duration::from_secs(30);

    if let Ok(entries) = fs::read_dir(download_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with(prefix) && name.ends_with(".csv") {
                            if let Ok(modified) = metadata.modified() {
                                if modified >= threshold {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}
"""

if fn_code in content:
    content = content.replace(fn_code, "")

    # insert before #[cfg(test)]
    tests_marker = "#[cfg(test)]"
    idx = content.find(tests_marker)

    if idx != -1:
        new_content = content[:idx] + fn_code + "\n" + content[idx:]
        with open('src/crm/fetcher.rs', 'w') as f:
            f.write(new_content)
    else:
        print("Could not find tests_marker")
else:
    print("Could not find fn_code")
