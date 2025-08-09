//! Interactive output handler implementation for UI integration
//! Delegates all output behavior to CliOutputHandler while maintaining UI integration

use trae_agent_core::output::{AgentOutput, AgentEvent};
use super::cli_handler::{CliOutputHandler, CliOutputConfig};
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Message types for interactive UI updates
#[derive(Debug, Clone)]
pub enum InteractiveMessage {
    /// Agent thinking/reasoning output
    AgentThinking(String),
    /// Tool execution status update
    ToolStatus(String),
    /// Tool execution result
    ToolResult(String),
    /// System message
    SystemMessage(String),
    /// Task completion
    TaskCompleted { success: bool, summary: String },
    /// Execution statistics
    ExecutionStats { steps: usize, duration: f64, tokens: Option<String> },
}

/// Interactive output configuration
#[derive(Debug, Clone)]
pub struct InteractiveOutputConfig {
    /// Whether to support real-time updates
    pub realtime_updates: bool,
    /// Whether to show detailed tool output
    pub show_tool_details: bool,
}

impl Default for InteractiveOutputConfig {
    fn default() -> Self {
        Self {
            realtime_updates: true,
            show_tool_details: true,
        }
    }
}

/// Interactive output handler that delegates all behavior to CliOutputHandler
/// while maintaining UI integration capabilities
pub struct InteractiveOutputHandler {
    /// The underlying CLI output handler that does the actual work
    cli_handler: CliOutputHandler,
    /// Channel sender for UI updates (optional for future use)
    _ui_sender: Option<mpsc::UnboundedSender<InteractiveMessage>>,
}

impl InteractiveOutputHandler {
    /// Create a new interactive output handler
    pub fn new(
        config: InteractiveOutputConfig,
        ui_sender: mpsc::UnboundedSender<InteractiveMessage>,
    ) -> Self {
        // Create CLI output handler with the same realtime_updates setting
        let cli_config = CliOutputConfig {
            realtime_updates: config.realtime_updates,
        };
        let cli_handler = CliOutputHandler::new(cli_config);

        Self {
            cli_handler,
            _ui_sender: Some(ui_sender),
        }
    }

    /// Create with default configuration
    pub fn with_sender(ui_sender: mpsc::UnboundedSender<InteractiveMessage>) -> Self {
        Self::new(InteractiveOutputConfig::default(), ui_sender)
    }
}

#[async_trait]
impl AgentOutput for InteractiveOutputHandler {
    async fn emit_event(&self, event: AgentEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // First delegate to CLI handler for console output
        self.cli_handler.emit_event(event.clone()).await?;

        // Then send UI messages if we have a sender
        if let Some(ui_sender) = &self._ui_sender {
            match event {
                AgentEvent::ExecutionStarted { context } => {
                    let msg = format!("⏳ Executing task: {}", context.task);
                    let _ = ui_sender.send(InteractiveMessage::SystemMessage(msg));
                }

                AgentEvent::ExecutionCompleted { context, success, summary } => {
                    let _ = ui_sender.send(InteractiveMessage::TaskCompleted { success, summary: summary.clone() });

                    // Send execution statistics
                    let token_msg = if context.token_usage.total_tokens > 0 {
                        Some(format!("🪙 {} input + {} output = {} total tokens",
                            context.token_usage.input_tokens,
                            context.token_usage.output_tokens,
                            context.token_usage.total_tokens))
                    } else {
                        None
                    };

                    let _ = ui_sender.send(InteractiveMessage::ExecutionStats {
                        steps: context.current_step,
                        duration: context.execution_time.as_secs_f64(),
                        tokens: token_msg,
                    });
                }

                AgentEvent::ToolExecutionStarted { tool_info } => {
                    // Skip status display for silent tools like sequentialthinking
                    if !is_silent_tool(&tool_info.tool_name) {
                        let status_msg = format!("🔧 {}", tool_info.tool_name);
                        let _ = ui_sender.send(InteractiveMessage::ToolStatus(status_msg));
                    }
                }

                AgentEvent::ToolExecutionCompleted { tool_info } => {
                    // Skip all output for silent tools
                    if is_silent_tool(&tool_info.tool_name) {
                        return Ok(());
                    }

                    let status_msg = match tool_info.status {
                        trae_agent_core::output::ToolExecutionStatus::Success => {
                            format!("✅ {} completed", tool_info.tool_name)
                        }
                        trae_agent_core::output::ToolExecutionStatus::Error => {
                            format!("❌ {} failed", tool_info.tool_name)
                        }
                        _ => {
                            format!("🔧 {} executing", tool_info.tool_name)
                        }
                    };
                    let _ = ui_sender.send(InteractiveMessage::ToolStatus(status_msg));

                    // Send result content if available
                    if let Some(result) = &tool_info.result {
                        if !result.content.is_empty() {
                            let _ = ui_sender.send(InteractiveMessage::ToolResult(result.content.clone()));
                        }
                    }
                }

                AgentEvent::AgentThinking { thinking, .. } => {
                    let _ = ui_sender.send(InteractiveMessage::AgentThinking(thinking));
                }

                AgentEvent::Message { content, .. } => {
                    let _ = ui_sender.send(InteractiveMessage::SystemMessage(content));
                }

                _ => {
                    // Other events are handled by CLI output
                }
            }
        }

        Ok(())
    }

    fn supports_realtime_updates(&self) -> bool {
        // Delegate to CLI output handler
        self.cli_handler.supports_realtime_updates()
    }

    async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Delegate to CLI output handler
        self.cli_handler.flush().await
    }
}

/// Tools that should not display status indicators
static SILENT_TOOLS: &[&str] = &[
    "sequentialthinking",
];

/// Check if a tool should be silent (no status display)
fn is_silent_tool(tool_name: &str) -> bool {
    SILENT_TOOLS.contains(&tool_name)
}


