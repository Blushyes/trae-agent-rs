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
        // Delegate all behavior to the CLI output handler
        self.cli_handler.emit_event(event).await
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


