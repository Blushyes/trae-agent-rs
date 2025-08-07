//! Bash execution tool

use crate::error::Result;
use crate::tools::{Tool, ToolCall, ToolExample, ToolResult};
use crate::tools::utils::maybe_truncate;
use crate::impl_tool_factory;
use async_trait::async_trait;
use serde_json::json;
use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::{sleep, timeout, Duration};

/// A session of a bash shell
struct BashSession {
    process: Option<Child>,
    started: bool,
    timed_out: bool,
    command: String,
    output_delay: Duration,
    timeout: Duration,
    sentinel: String,
}

impl BashSession {
    fn new() -> Self {
        Self {
            process: None,
            started: false,
            timed_out: false,
            command: "/bin/bash".to_string(),
            output_delay: Duration::from_millis(200),
            timeout: Duration::from_secs(120),
            sentinel: ",,,,bash-command-exit-__ERROR_CODE__-banner,,,,".to_string(),
        }
    }

    async fn start(&mut self) -> Result<()> {
        if self.started {
            return Ok(());
        }

        let mut cmd = Command::new(&self.command);
        cmd.stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        // On Unix-like systems, set process group
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }

        self.process = Some(cmd.spawn()?);
        self.started = true;
        Ok(())
    }

    fn stop(&mut self) {
        if !self.started {
            return;
        }

        if let Some(mut process) = self.process.take() {
            if process.try_wait().unwrap_or(None).is_none() {
                let _ = process.kill();
            }
        }
        self.started = false;
    }

    async fn run(&mut self, command: &str) -> Result<(i32, String, String)> {
        if !self.started || self.process.is_none() {
            return Err("Session has not started.".into());
        }

        if self.timed_out {
            return Err(format!(
                "timed out: bash has not returned in {} seconds and must be restarted",
                self.timeout.as_secs()
            ).into());
        }

        let process = self.process.as_mut().unwrap();

        // Check if process is still alive
        if let Ok(Some(status)) = process.try_wait() {
            return Err(format!(
                "bash has exited with returncode {}. tool must be restarted.",
                status.code().unwrap_or(-1)
            ).into());
        }

        let _error_code = 0;
        let (sentinel_before, sentinel_after) = self.sentinel.split_once("__ERROR_CODE__")
            .ok_or("Invalid sentinel format")?;

        let errcode_retriever = "$?";
        let command_sep = ";";

        // Send command to the process
        if let Some(stdin) = process.stdin.as_mut() {
            let full_command = format!(
                "(\n{}\n){} echo {}\n",
                command,
                command_sep,
                self.sentinel.replace("__ERROR_CODE__", errcode_retriever)
            );
            stdin.write_all(full_command.as_bytes()).await?;
            stdin.flush().await?;
        } else {
            return Err("No stdin available".into());
        }

        // Read output from the process until sentinel is found
        let result = timeout(self.timeout, async {
            let mut output = String::new();
            let mut error_code = 0;

            if let Some(stdout) = process.stdout.as_mut() {
                let mut reader = BufReader::new(stdout);
                let mut buffer = Vec::new();

                loop {
                    sleep(self.output_delay).await;

                    // Try to read available data
                    match reader.read_until(b'\n', &mut buffer).await {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            let line = String::from_utf8_lossy(&buffer);
                            output.push_str(&line);
                            buffer.clear();

                            if output.contains(sentinel_before) {
                                // Extract the sentinel and error code
                                if let Some(pos) = output.rfind(sentinel_before) {
                                    let content = output[..pos].to_string();
                                    let rest = &output[pos..];

                                    // Extract error code from the sentinel
                                    if let Some(code_start) = rest.find(sentinel_before) {
                                        let code_part = &rest[code_start + sentinel_before.len()..];
                                        if let Some(code_end) = code_part.find(sentinel_after) {
                                            let code_str = &code_part[..code_end];
                                            error_code = code_str.trim().parse().unwrap_or(-1);
                                        }
                                    }

                                    output = content;
                                    break;
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            }

            Ok::<(i32, String, String), crate::error::Error>((error_code, output, String::new()))
        }).await;

        match result {
            Ok(Ok((exit_code, stdout, stderr))) => {
                let stdout_clean = if stdout.ends_with('\n') {
                    stdout.trim_end_matches('\n').to_string()
                } else {
                    stdout
                };
                Ok((exit_code, stdout_clean, stderr))
            }
            Ok(Err(e)) => Err(e),
            Err(_) => {
                self.timed_out = true;
                Err(format!(
                    "timed out: bash has not returned in {} seconds and must be restarted",
                    self.timeout.as_secs()
                ).into())
            }
        }
    }
}

/// Tool for executing bash commands with session management
pub struct BashTool {
    session: Arc<Mutex<Option<BashSession>>>,
}

impl BashTool {
    /// Create a new bash tool
    pub fn new() -> Self {
        Self {
            session: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Run commands in a bash shell\n\
         * When invoking this tool, the contents of the \"command\" parameter does NOT need to be XML-escaped.\n\
         * You have access to a mirror of common linux and python packages via apt and pip.\n\
         * State is persistent across command calls and discussions with the user.\n\
         * To inspect a particular line range of a file, e.g. lines 10-25, try 'sed -n 10,25p /path/to/the/file'.\n\
         * Please avoid commands that may produce a very large amount of output.\n\
         * Please run long lived commands in the background, e.g. 'sleep 10 &' or start a server in the background."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The bash command to run."
                },
                "restart": {
                    "type": "boolean",
                    "description": "Set to true to restart the bash session."
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let restart: bool = call.get_parameter_or("restart", false);

        if restart {
            // Handle restart without holding lock across await
            {
                let mut session_guard = self.session.lock().await;
                if let Some(ref mut session) = *session_guard {
                    session.stop();
                }
                *session_guard = Some(BashSession::new());
            }

            // Start the new session
            {
                let mut session_guard = self.session.lock().await;
                if let Some(ref mut session) = *session_guard {
                    session.start().await?;
                }
            }

            return Ok(ToolResult::success(&call.id, &"tool has been restarted.".to_string()));
        }

        let command: String = call.get_parameter("command")?;

        // Ensure session exists and is started
        let needs_start = {
            let mut session_guard = self.session.lock().await;
            if session_guard.is_none() {
                *session_guard = Some(BashSession::new());
                true
            } else if let Some(ref session) = *session_guard {
                !session.started
            } else {
                false
            }
        };

        if needs_start {
            let mut session_guard = self.session.lock().await;
            if let Some(ref mut session) = *session_guard {
                session.start().await?;
            }
        }

        // Execute command
        let result = {
            let mut session_guard = self.session.lock().await;
            if let Some(ref mut session) = *session_guard {
                session.run(&command).await
            } else {
                return Err("No session available".into());
            }
        };

        match result {
            Ok((exit_code, stdout, stderr)) => {
                let mut output = String::new();

                if !stdout.is_empty() {
                    output.push_str(&maybe_truncate(&stdout, None));
                }

                if !stderr.is_empty() {
                    if !output.is_empty() {
                        output.push_str("\n");
                    }
                    output.push_str(&maybe_truncate(&stderr, None));
                }

                if output.is_empty() {
                    output = format!("Command completed with exit code: {}", exit_code);
                }

                Ok(ToolResult::success(&call.id, &output).with_data(json!({
                    "exit_code": exit_code,
                    "stdout": stdout,
                    "stderr": stderr
                })))
            }
            Err(e) => {
                Ok(ToolResult::error(&call.id, &format!("Error running bash command: {}", e)))
            }
        }
    }

    fn requires_confirmation(&self) -> bool {
        true // Bash commands can be dangerous
    }
    
    fn examples(&self) -> Vec<ToolExample> {
        vec![
            ToolExample {
                description: "List files in current directory".to_string(),
                parameters: json!({"command": "ls -la"}),
                expected_result: "Directory listing with file details".to_string(),
            },
            ToolExample {
                description: "Check Python version".to_string(),
                parameters: json!({"command": "python --version"}),
                expected_result: "Python version information".to_string(),
            },
            ToolExample {
                description: "Restart bash session".to_string(),
                parameters: json!({"command": "echo 'restarting'", "restart": true}),
                expected_result: "Session restarted message".to_string(),
            },
            ToolExample {
                description: "Run a command with persistent state".to_string(),
                parameters: json!({"command": "export MY_VAR=hello && echo $MY_VAR"}),
                expected_result: "Variable set and echoed in persistent session".to_string(),
            },
        ]
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

impl_tool_factory!(
    BashToolFactory,
    BashTool,
    "bash",
    "Execute shell commands in a bash environment"
);
