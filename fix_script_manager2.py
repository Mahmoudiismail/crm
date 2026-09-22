import sys

path = 'src/tasker/script_manager.rs'
content = open(path, 'r').read()

old_func = """    pub fn execute_script_with_args(
        &self,
        script_path: &Path,
        args: &[(&str, &str)],
    ) -> Result<()> {
        if !script_path.exists() {
            anyhow::bail!("PowerShell script file does not exist at {:?}", script_path);
        }

        info!(
            "Executing persistent PowerShell script: {:?} with args {:?}",
            script_path, args
        );

        let mut cmd = std::process::Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_path.to_str().ok_or_else(|| {
                anyhow::anyhow!("Invalid unicode path for script {:?}", script_path)
            })?,
        ]);

        for (k, v) in args {
            cmd.arg(k).arg(v);
        }

        let output = cmd
            .stdin(Stdio::null())
            .output()
            .context("Failed to spawn PowerShell process")?;

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);

        if !stdout_str.trim().is_empty() {
            for line in stdout_str.lines() {
                if line.starts_with("TRACE:") {
                    trace!("PS: {}", line.strip_prefix("TRACE:").unwrap_or(line).trim());
                } else if !line.trim().is_empty() {
                    info!("PS: {}", line.trim());
                }
            }
        }

        if !stderr_str.trim().is_empty() {
            for line in stderr_str.lines() {
                error!("PS ERROR: {}", line.trim());
            }
        }

        if !output.status.success() {
            anyhow::bail!(
                "PowerShell script at {:?} failed with status: {}",
                script_path,
                output.status
            );
        }

        Ok(())
    }"""

new_func = """    pub fn execute_script_with_args(
        &self,
        script_path: &Path,
        args: &[(&str, &str)],
    ) -> Result<()> {
        if !script_path.exists() {
            anyhow::bail!("PowerShell script file does not exist at {:?}", script_path);
        }

        let mut safe_args = Vec::new();
        for (k, v) in args {
            let k_lower = k.to_lowercase();
            if k_lower.contains("htmlbody") || k_lower.contains("emailto") || k_lower.contains("csvpath") || k_lower.contains("subject") {
                safe_args.push((*k, "<REDACTED>"));
            } else {
                safe_args.push((*k, *v));
            }
        }

        info!(
            "Executing persistent PowerShell script: {:?} with args {:?}",
            script_path, safe_args
        );

        let mut cmd = std::process::Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script_path.to_str().ok_or_else(|| {
                anyhow::anyhow!("Invalid unicode path for script {:?}", script_path)
            })?,
        ]);

        for (k, v) in args {
            cmd.arg(k).arg(v);
        }

        let mut child = cmd
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn PowerShell process")?;

        let pid = child.id();
        let timeout_duration = std::time::Duration::from_secs(5 * 60);
        let start_time = std::time::Instant::now();

        let mut stdout_bytes = Vec::new();
        let mut stderr_bytes = Vec::new();
        let max_output_size = 10 * 1024 * 1024; // 10MB

        use std::io::Read;
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        let (tx2, rx2) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let mut buf = [0; 4096];
            loop {
                match stdout.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => if tx.send(buf[..n].to_vec()).is_err() { break; },
                    Err(_) => break,
                }
            }
        });

        std::thread::spawn(move || {
            let mut buf = [0; 4096];
            loop {
                match stderr.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => if tx2.send(buf[..n].to_vec()).is_err() { break; },
                    Err(_) => break,
                }
            }
        });

        let mut timed_out = false;
        loop {
            if let Ok(Some(status)) = child.try_wait() {
                while let Ok(chunk) = rx.try_recv() {
                    if stdout_bytes.len() + chunk.len() <= max_output_size {
                        stdout_bytes.extend_from_slice(&chunk);
                    }
                }
                while let Ok(chunk) = rx2.try_recv() {
                    if stderr_bytes.len() + chunk.len() <= max_output_size {
                        stderr_bytes.extend_from_slice(&chunk);
                    }
                }

                let stdout_str = String::from_utf8_lossy(&stdout_bytes);
                let stderr_str = String::from_utf8_lossy(&stderr_bytes);

                if !stdout_str.trim().is_empty() {
                    for line in stdout_str.lines() {
                        if line.starts_with("TRACE:") {
                            trace!("PS: {}", line.strip_prefix("TRACE:").unwrap_or(line).trim());
                        } else if !line.trim().is_empty() {
                            info!("PS: {}", line.trim());
                        }
                    }
                }

                if !stderr_str.trim().is_empty() {
                    for line in stderr_str.lines() {
                        error!("PS ERROR: {}", line.trim());
                    }
                }

                if !status.success() {
                    anyhow::bail!(
                        "PowerShell script at {:?} failed with status: {}",
                        script_path,
                        status
                    );
                }
                break;
            }

            if start_time.elapsed() > timeout_duration {
                timed_out = true;
                break;
            }

            while let Ok(chunk) = rx.try_recv() {
                if stdout_bytes.len() + chunk.len() <= max_output_size {
                    stdout_bytes.extend_from_slice(&chunk);
                }
            }
            while let Ok(chunk) = rx2.try_recv() {
                if stderr_bytes.len() + chunk.len() <= max_output_size {
                    stderr_bytes.extend_from_slice(&chunk);
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        if timed_out {
            let _ = std::process::Command::new("taskkill")
                .args(&["/F", "/T", "/PID", &pid.to_string()])
                .output();
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("PowerShell script execution timed out after 5 minutes");
        }

        Ok(())
    }"""

content = content.replace(old_func, new_func)

open(path, 'w').write(content)
