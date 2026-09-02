import re

with open("src/yasweb/browser/download.rs", "r") as f:
    content = f.read()

# Change the polling loop to use time::Instant so we can poll every 200ms instead of every 1s
wait_loop = """
        let mut download_complete = false;
        let timeout_seconds = timeout_minutes * 60;

        for _ in 0..timeout_seconds {
            if let Ok(entries) = std::fs::read_dir(dl_dir) {
                let mut found_incomplete = false;
                let mut found_completed = false;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "crdownload" || ext == "tmp" {
                            found_incomplete = true;
                        } else if ext == "xlsx" || ext == "xls" || ext == "csv" {
                            found_completed = true;
                        }
                    }
                }

                if found_completed && !found_incomplete {
                    download_complete = true;
                    break;
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        }
"""

replacement_wait_loop = """
        let mut download_complete = false;
        let timeout_duration = Duration::from_secs(timeout_minutes * 60);
        let start_time = std::time::Instant::now();

        while start_time.elapsed() < timeout_duration {
            if let Ok(entries) = std::fs::read_dir(dl_dir) {
                let mut found_incomplete = false;
                let mut found_completed = false;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "crdownload" || ext == "tmp" {
                            found_incomplete = true;
                        } else if ext == "xlsx" || ext == "xls" || ext == "csv" {
                            found_completed = true;
                        }
                    }
                }

                if found_completed && !found_incomplete {
                    download_complete = true;
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(200));
        }
"""

content = content.replace(wait_loop, replacement_wait_loop)

with open("src/yasweb/browser/download.rs", "w") as f:
    f.write(content)
