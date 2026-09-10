use crate::runner::engine::logging::TaskLogger;
use crate::runner::engine::process::{run_process, ProcessContext};
use anyhow::Result;

pub fn parse_command_args(command: &str) -> Option<Vec<String>> {
    let s = command.trim();
    if s.is_empty() {
        return None;
    }

    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            '\\' if !in_single => {
                if let Some(&next) = chars.peek() {
                    if in_double {
                        if next == '"' || next == '\\' {
                            chars.next();
                            current.push(next);
                        } else {
                            current.push('\\');
                        }
                    } else {
                        if next == '"' || next == '\'' || next.is_whitespace() || next == '\\' {
                            chars.next();
                            current.push(next);
                        } else {
                            current.push('\\');
                        }
                    }
                } else {
                    current.push('\\');
                }
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if in_single || in_double {
        return None;
    }

    if !current.is_empty() {
        args.push(current);
    }

    if args.is_empty() {
        None
    } else {
        Some(args)
    }
}

#[cfg(target_os = "windows")]
fn is_cmd_builtin(cmd: &str) -> bool {
    let lc = cmd.to_lowercase();
    matches!(
        lc.as_str(),
        "assoc"
            | "break"
            | "call"
            | "cd"
            | "chcp"
            | "chdir"
            | "cls"
            | "color"
            | "copy"
            | "date"
            | "del"
            | "dir"
            | "echo"
            | "erase"
            | "exit"
            | "ftype"
            | "md"
            | "mkdir"
            | "move"
            | "path"
            | "pause"
            | "prompt"
            | "rd"
            | "ren"
            | "rename"
            | "rmdir"
            | "set"
            | "start"
            | "time"
            | "title"
            | "type"
            | "ver"
            | "vol"
    )
}

pub async fn run_shell_command(
    logger: &TaskLogger,
    command: &str,
    shell_timeout_seconds: u64,
) -> Result<()> {
    let args = parse_command_args(command).ok_or_else(|| {
        anyhow::anyhow!("Failed to parse command string: invalid quoting or syntax")
    })?;

    #[cfg(target_os = "windows")]
    let cmd = {
        let prog = &args[0];
        if is_cmd_builtin(prog) {
            let mut c = tokio::process::Command::new("cmd.exe");
            c.arg("/c").arg(prog).args(&args[1..]);
            c
        } else {
            let mut c = tokio::process::Command::new(prog);
            c.args(&args[1..]);
            c
        }
    };

    #[cfg(not(target_os = "windows"))]
    let cmd = {
        let mut c = tokio::process::Command::new("bash");
        c.arg("-c").arg("\"$@\"").arg("--").args(&args);
        c
    };

    let ctx = ProcessContext {
        logger,
        command_str: format!("shell command: {}", command),
        timeout_seconds: shell_timeout_seconds,
        cmd,
    };

    run_process(ctx)
        .await
        .map_err(|e| anyhow::anyhow!("Command failed: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_args_windows_paths() {
        let parsed =
            parse_command_args(r#""C:\Program Files\App\app.exe" --file "C:\data\file.txt""#)
                .unwrap();
        assert_eq!(
            parsed,
            vec![
                r#"C:\Program Files\App\app.exe"#,
                "--file",
                r#"C:\data\file.txt"#
            ]
        );
    }

    #[tokio::test]
    async fn test_empty_shell_command_fails() {
        let logger = TaskLogger::new("test_task", "Test Task");
        let result = run_shell_command(&logger, "   ", 5).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid quoting or syntax"));
    }

    #[tokio::test]
    async fn test_invalid_quoting_fails() {
        let logger = TaskLogger::new("test_task", "Test Task");
        let result = run_shell_command(&logger, "echo 'unclosed quote", 5).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid quoting or syntax"));
    }

    #[tokio::test]
    async fn test_valid_shell_command_executes() {
        let logger = TaskLogger::new("test_task", "Test Task");
        let result = run_shell_command(&logger, "echo hello", 5).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metacharacters_prevent_injection() {
        let logger = TaskLogger::new("test_task", "Test Task");
        // Command with semicolon or subshell attempt should be passed literally to echo
        let result = run_shell_command(&logger, "echo 'hello; calc; $(id)'", 5).await;
        assert!(result.is_ok());
    }
}
