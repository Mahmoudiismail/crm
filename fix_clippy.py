import sys

def fix_file(path):
    content = open(path, "r").read()

    # Fix manual_flatten for dashboard_updater and crm_open_sohail
    old_stdout_loop = """        for line in reader.lines() {
            if let Ok(l) = line {
                if !l.trim().is_empty() {
                    info!("PowerShell: {}", l);
                }
            }
        }"""
    new_stdout_loop = """        for l in reader.lines().flatten() {
            if !l.trim().is_empty() {
                info!("PowerShell: {}", l);
            }
        }"""
    content = content.replace(old_stdout_loop, new_stdout_loop)

    old_stdout_loop_tracing = """        for line in reader.lines() {
            if let Ok(l) = line {
                if !l.trim().is_empty() {
                    tracing::info!("PowerShell: {}", l);
                }
            }
        }"""
    new_stdout_loop_tracing = """        for l in reader.lines().flatten() {
            if !l.trim().is_empty() {
                tracing::info!("PowerShell: {}", l);
            }
        }"""
    content = content.replace(old_stdout_loop_tracing, new_stdout_loop_tracing)

    old_stderr_loop = """        for line in reader.lines() {
            if let Ok(l) = line {
                if !l.trim().is_empty() {
                    error!("PowerShell Error: {}", l);
                }
            }
        }"""
    new_stderr_loop = """        for l in reader.lines().flatten() {
            if !l.trim().is_empty() {
                error!("PowerShell Error: {}", l);
            }
        }"""
    content = content.replace(old_stderr_loop, new_stderr_loop)

    old_stderr_loop_tracing = """        for line in reader.lines() {
            if let Ok(l) = line {
                if !l.trim().is_empty() {
                    tracing::error!("PowerShell Error: {}", l);
                }
            }
        }"""
    new_stderr_loop_tracing = """        for l in reader.lines().flatten() {
            if !l.trim().is_empty() {
                tracing::error!("PowerShell Error: {}", l);
            }
        }"""
    content = content.replace(old_stderr_loop_tracing, new_stderr_loop_tracing)

    # Fix collapsible_if for crm_open_sohail
    old_headers = """    for (i, h) in headers.iter().enumerate() {
        let h_lower = h.trim().to_lowercase();
        if h_lower == "team name" || h_lower == "team" || h_lower.contains("team") {
            if team_idx.is_none() { team_idx = Some(i); }
        } else if h_lower == "owner" || h_lower == "receiver name" || h_lower == "oul" || h_lower.contains("owner") || h_lower.contains("receiver") {
            if owner_idx.is_none() { owner_idx = Some(i); }
        } else if h_lower == "to emails" || h_lower == "email" || h_lower == "email_to" || h_lower.contains("email") {
            if email_idx.is_none() { email_idx = Some(i); }
        }
    }"""
    new_headers = """    for (i, h) in headers.iter().enumerate() {
        let h_lower = h.trim().to_lowercase();
        if (h_lower == "team name" || h_lower == "team" || h_lower.contains("team")) && team_idx.is_none() {
            team_idx = Some(i);
        } else if (h_lower == "owner" || h_lower == "receiver name" || h_lower == "oul" || h_lower.contains("owner") || h_lower.contains("receiver")) && owner_idx.is_none() {
            owner_idx = Some(i);
        } else if (h_lower == "to emails" || h_lower == "email" || h_lower == "email_to" || h_lower.contains("email")) && email_idx.is_none() {
            email_idx = Some(i);
        }
    }"""
    content = content.replace(old_headers, new_headers)

    open(path, "w").write(content)

fix_file("src/tasker/dashboard_updater.rs")
fix_file("src/tasker/crm_open_sohail.rs")
print("Fixed clippy errors")
