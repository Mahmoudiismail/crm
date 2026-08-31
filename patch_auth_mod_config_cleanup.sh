#!/bin/bash
set -e

# Update config.rs
sed -i 's/pub last_run_timestamp: u64,/pub last_run_timestamp: u64,\n    #[serde(default, skip_serializing_if = "Option::is_none")]\n    pub retention_days: Option<u32>,/' src/crm/config.rs
sed -i 's/last_run_timestamp: 0,/last_run_timestamp: 0,\n            retention_days: None,/' src/crm/config.rs
sed -i 's/.field("last_run_timestamp", &self.last_run_timestamp)/.field("last_run_timestamp", \&self.last_run_timestamp)\n            .field("retention_days", \&self.retention_days)/' src/crm/config.rs

# Create cleanup.rs
cat << 'CLEANUP_EOF' > src/crm/cleanup.rs
use std::fs;
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tracing::info;

pub fn cleanup_old_reports(download_dir: &Path, retention_days: u32) -> Result<usize, io::Error> {
    if retention_days == 0 {
        return Ok(0);
    }

    let mut deleted_count = 0;
    let threshold = SystemTime::now() - Duration::from_secs(retention_days as u64 * 86400);

    for entry in fs::read_dir(download_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if (file_name.starts_with("ticket_report_") || file_name.starts_with("lead_report_"))
                    && file_name.ends_with(".csv") {

                    let metadata = fs::metadata(&path)?;
                    if let Ok(modified) = metadata.modified() {
                        if modified < threshold {
                            fs::remove_file(&path)?;
                            info!("Deleted old report file: {:?}", path);
                            deleted_count += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(deleted_count)
}
CLEANUP_EOF

# Update mod.rs
sed -i 's/pub mod auth;/pub mod auth;\npub mod cleanup;/' src/crm/mod.rs
sed -i 's/auth::ensure_authenticated(config, &client, false)/auth::ensure_authenticated(config, \&client, false, false)/' src/crm/mod.rs

cat << 'MOD_EOF' > patch_mod.diff
--- src/crm/mod.rs
+++ src/crm/mod.rs
@@ -90,6 +90,16 @@
         let final_cfg = config_arc.lock().await;
         *config = final_cfg.clone();
     }
+
+    if let Some(retention_days) = config.retention_days {
+        if retention_days > 0 {
+            tracing::info!("Running auto-cleanup for reports older than {} days...", retention_days);
+            match crate::crm::cleanup::cleanup_old_reports(&download_dir, retention_days) {
+                Ok(deleted_count) => tracing::info!("Cleanup completed. Deleted {} old files.", deleted_count),
+                Err(e) => tracing::error!("Failed to clean up old reports: {:?}", e),
+            }
+        }
+    }

     use std::time::{SystemTime, UNIX_EPOCH};
     config.last_run_timestamp = SystemTime::now()
MOD_EOF
patch src/crm/mod.rs patch_mod.diff
