//! Tool execution utilities

use crate::error::Result;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::{timeout, Duration, Instant};

/// Command execution options
#[derive(Debug, Clone)]
pub struct CommandOptions {
    pub timeout_seconds: Option<u64>,
    pub truncate_after: Option<usize>,
    pub working_directory: Option<String>,
    pub environment: HashMap<String, String>,
    pub capture_stderr: bool,
    pub shell: Option<String>,
}

impl Default for CommandOptions {
    fn default() -> Self {
        Self {
            timeout_seconds: Some(120),
            truncate_after: Some(16000),
            working_directory: None,
            environment: HashMap::new(),
            capture_stderr: true,
            shell: Some("/bin/bash".to_string()),
        }
    }
}

/// Command execution result
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
    pub truncated: bool,
}

/// Execute a command with comprehensive options
pub async fn execute_command(command: &str, options: CommandOptions) -> Result<CommandResult> {
    let start_time = Instant::now();
    
    let mut cmd = if let Some(shell) = &options.shell {
        let mut cmd = Command::new(shell);
        cmd.arg("-c").arg(command);
        cmd
    } else {
        // Parse command and arguments
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".into());
        }
        
        let mut cmd = Command::new(parts[0]);
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }
        cmd
    };

    // Set working directory
    if let Some(working_dir) = &options.working_directory {
        cmd.current_dir(working_dir);
    }

    // Set environment variables
    for (key, value) in &options.environment {
        cmd.env(key, value);
    }

    // Configure stdio
    cmd.stdin(Stdio::piped())
       .stdout(Stdio::piped());
    
    if options.capture_stderr {
        cmd.stderr(Stdio::piped());
    } else {
        cmd.stderr(Stdio::inherit());
    }

    let mut child = cmd.spawn()?;
    
    // Execute with timeout
    let timeout_duration = Duration::from_secs(options.timeout_seconds.unwrap_or(120));
    let result = timeout(timeout_duration, async {
        execute_child(&mut child, options.capture_stderr).await
    }).await;

    let duration = start_time.elapsed();
    
    match result {
        Ok(Ok((exit_code, stdout, stderr))) => {
            let truncate_limit = options.truncate_after.unwrap_or(16000);
            let (stdout_truncated, stdout_final) = truncate_output(&stdout, truncate_limit);
            let (stderr_truncated, stderr_final) = truncate_output(&stderr, truncate_limit);
            
            Ok(CommandResult {
                exit_code,
                stdout: stdout_final,
                stderr: stderr_final,
                duration_ms: duration.as_millis() as u64,
                timed_out: false,
                truncated: stdout_truncated || stderr_truncated,
            })
        }
        Ok(Err(e)) => Err(e),
        Err(_) => {
            // Kill the process if it's still running
            let _ = child.kill().await;
            
            Ok(CommandResult {
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Command timed out after {} seconds", timeout_duration.as_secs()),
                duration_ms: duration.as_millis() as u64,
                timed_out: true,
                truncated: false,
            })
        }
    }
}

/// Execute child process and capture output
async fn execute_child(child: &mut Child, capture_stderr: bool) -> Result<(i32, String, String)> {
    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = if capture_stderr {
        child.stderr.take()
    } else {
        None
    };

    let mut stdout_reader = BufReader::new(stdout);
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();

    // Read stdout
    let stdout_task = async {
        let mut line = String::new();
        while stdout_reader.read_line(&mut line).await? > 0 {
            stdout_lines.push(line.clone());
            line.clear();
        }
        Ok::<(), std::io::Error>(())
    };

    // Read stderr if capturing
    let stderr_task = async {
        if let Some(stderr) = stderr {
            let mut stderr_reader = BufReader::new(stderr);
            let mut line = String::new();
            while stderr_reader.read_line(&mut line).await? > 0 {
                stderr_lines.push(line.clone());
                line.clear();
            }
        }
        Ok::<(), std::io::Error>(())
    };

    // Wait for both tasks to complete
    let (stdout_result, stderr_result) = tokio::join!(stdout_task, stderr_task);
    stdout_result?;
    stderr_result?;

    // Wait for process to exit
    let status = child.wait().await?;
    let exit_code = status.code().unwrap_or(-1);

    let stdout_output = stdout_lines.join("");
    let stderr_output = stderr_lines.join("");

    Ok((exit_code, stdout_output, stderr_output))
}

/// Truncate output if it exceeds the limit
fn truncate_output(output: &str, limit: usize) -> (bool, String) {
    if output.len() <= limit {
        (false, output.to_string())
    } else {
        let truncated = format!(
            "{}\n\n<output truncated after {} characters>\n\
             <NOTE>To see the full output, increase the truncate_after limit or \
             redirect output to a file.</NOTE>",
            &output[..limit],
            limit
        );
        (true, truncated)
    }
}

/// Stream command output in real-time
pub async fn stream_command(
    command: &str,
    options: CommandOptions,
    mut output_handler: impl FnMut(&str) -> Result<()>,
) -> Result<CommandResult> {
    let start_time = Instant::now();
    
    let mut cmd = if let Some(shell) = &options.shell {
        let mut cmd = Command::new(shell);
        cmd.arg("-c").arg(command);
        cmd
    } else {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".into());
        }
        
        let mut cmd = Command::new(parts[0]);
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }
        cmd
    };

    if let Some(working_dir) = &options.working_directory {
        cmd.current_dir(working_dir);
    }

    for (key, value) in &options.environment {
        cmd.env(key, value);
    }

    cmd.stdin(Stdio::piped())
       .stdout(Stdio::piped())
       .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    
    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    let mut stdout_reader = BufReader::new(stdout);
    let mut stderr_reader = BufReader::new(stderr);
    
    let mut all_output = String::new();
    let mut exit_code = 0;
    let mut timed_out = false;

    let timeout_duration = Duration::from_secs(options.timeout_seconds.unwrap_or(120));
    let result = timeout(timeout_duration, async {
        let mut stdout_line = String::new();
        let mut stderr_line = String::new();

        loop {
            tokio::select! {
                result = stdout_reader.read_line(&mut stdout_line) => {
                    match result {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            output_handler(&stdout_line)?;
                            all_output.push_str(&stdout_line);
                            stdout_line.clear();
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                result = stderr_reader.read_line(&mut stderr_line) => {
                    match result {
                        Ok(0) => {}, // EOF on stderr
                        Ok(_) => {
                            output_handler(&stderr_line)?;
                            all_output.push_str(&stderr_line);
                            stderr_line.clear();
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                status = child.wait() => {
                    exit_code = status?.code().unwrap_or(-1);
                    break;
                }
            }
        }

        Ok::<(), crate::error::Error>(())
    }).await;

    let duration = start_time.elapsed();

    match result {
        Ok(Ok(())) => {},
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            let _ = child.kill().await;
            timed_out = true;
            exit_code = -1;
        }
    }

    let truncate_limit = options.truncate_after.unwrap_or(16000);
    let (truncated, final_output) = truncate_output(&all_output, truncate_limit);

    Ok(CommandResult {
        exit_code,
        stdout: final_output,
        stderr: String::new(), // Combined with stdout in streaming mode
        duration_ms: duration.as_millis() as u64,
        timed_out,
        truncated,
    })
}

/// Validate command safety (basic checks)
pub fn validate_command_safety(command: &str) -> Result<()> {
    let dangerous_patterns = [
        "rm -rf /",
        ":(){ :|:& };:",  // Fork bomb
        "dd if=/dev/zero",
        "mkfs.",
        "format ",
        "> /dev/",
        "chmod 777 /",
        "chown root /",
    ];

    let command_lower = command.to_lowercase();
    for pattern in &dangerous_patterns {
        if command_lower.contains(pattern) {
            return Err(format!("Potentially dangerous command detected: {}", pattern).into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_command() {
        let options = CommandOptions::default();
        let result = execute_command("echo 'Hello, World!'", options).await.unwrap();
        
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("Hello, World!"));
        assert!(!result.timed_out);
    }

    #[tokio::test]
    async fn test_command_timeout() {
        let mut options = CommandOptions::default();
        options.timeout_seconds = Some(1);
        
        let result = execute_command("sleep 5", options).await.unwrap();
        
        assert!(result.timed_out);
        assert_eq!(result.exit_code, -1);
    }

    #[test]
    fn test_output_truncation() {
        let long_output = "a".repeat(20000);
        let (truncated, output) = truncate_output(&long_output, 1000);
        
        assert!(truncated);
        assert!(output.len() > 1000); // Includes truncation message
        assert!(output.contains("output truncated"));
    }

    #[test]
    fn test_command_safety_validation() {
        assert!(validate_command_safety("echo hello").is_ok());
        assert!(validate_command_safety("rm -rf /").is_err());
        assert!(validate_command_safety("dd if=/dev/zero of=/dev/sda").is_err());
    }
}
