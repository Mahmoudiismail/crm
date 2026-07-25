use std::path::PathBuf;

pub fn resolve_executable(configured: &str) -> PathBuf {
    let configured = configured.trim();
    let configured_name = if configured.is_empty() {
        default_crm_binary_name().to_string()
    } else {
        configured.to_string()
    };

    let configured_path = PathBuf::from(&configured_name);
    if configured_path.is_absolute() {
        return configured_path;
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let sibling = exe_dir.join(&configured_name);
            if sibling.exists() {
                return sibling;
            }

            if configured.is_empty() {
                let default_sibling = exe_dir.join(default_crm_binary_name());
                if default_sibling.exists() {
                    return default_sibling;
                }
            }
        }
    }

    configured_path
}

pub fn resolve_relative_to_exe_dir(path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        return p;
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            return exe_dir.join(p);
        }
    }

    p
}

fn default_crm_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "crm.exe"
    } else {
        "crm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_relative_to_exe_dir_absolute_path() {
        // Use an absolute path based on the OS
        let absolute_path = if cfg!(target_os = "windows") {
            "C:\\foo\\bar"
        } else {
            "/foo/bar"
        };
        let resolved = resolve_relative_to_exe_dir(absolute_path);
        assert_eq!(resolved, std::path::PathBuf::from(absolute_path));
    }

    #[test]
    fn test_resolve_relative_to_exe_dir_relative_path() {
        let relative_path = "config.json";
        let resolved = resolve_relative_to_exe_dir(relative_path);

        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let expected = exe_dir.join(relative_path);

        assert_eq!(resolved, expected);
    }

    #[test]
    fn test_resolve_relative_to_exe_dir_dot_path() {
        let dot_path = ".";
        let resolved = resolve_relative_to_exe_dir(dot_path);

        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let expected = exe_dir.join(dot_path);

        assert_eq!(resolved, expected);
    }
}
