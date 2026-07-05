import re

with open('src/crm/fetcher.rs', 'r') as f:
    content = f.read()

# Determine prefix for each report type
# "tickets" -> "ticket_report_"
# "calls" -> "call_logs_"
# "leads" -> "lead_report_"

# Replace the task building block inside `for def in defs {`
old_block = """        if !should_fetch(def.key) {
            continue;
        }

        let endpoint = def.endpoint;
        let extra = def.extra_params;"""

new_block = """        if !should_fetch(def.key) {
            continue;
        }

        let prefix = match def.key {
            "tickets" => "ticket_report_",
            "calls" => "call_logs_",
            "leads" => "lead_report_",
            _ => "",
        };

        if !prefix.is_empty() && has_recent_download(download_dir, prefix) {
            info!("Skipping fetch for '{}': A recent file (<30s old) already exists in Downloads", def.key);
            continue;
        }

        let endpoint = def.endpoint;
        let extra = def.extra_params;"""

content = content.replace(old_block, new_block)

with open('src/crm/fetcher.rs', 'w') as f:
    f.write(content)
