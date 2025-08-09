//! Interactive output handler implementation for UI integration

use trae_agent_core::output::{
    AgentOutput, AgentEvent, ToolExecutionStatus, MessageLevel
};
use super::formatters::{ToolFormatter, DiffFormatter};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, info, warn, error};

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

/// Interactive output handler that sends formatted events to UI components
pub struct InteractiveOutputHandler {
    config: InteractiveOutputConfig,
    tool_formatter: ToolFormatter,
    diff_formatter: DiffFormatter,
    /// Channel sender for UI updates
    ui_sender: mpsc::UnboundedSender<InteractiveMessage>,
    /// Track active tool executions for real-time updates
    active_tools: Arc<Mutex<HashMap<String, trae_agent_core::output::ToolExecutionInfo>>>,
}

impl InteractiveOutputHandler {
    /// Create a new interactive output handler
    pub fn new(
        config: InteractiveOutputConfig,
        ui_sender: mpsc::UnboundedSender<InteractiveMessage>,
    ) -> Self {
        Self {
            config,
            tool_formatter: ToolFormatter::new(),
            diff_formatter: DiffFormatter::new(),
            ui_sender,
            active_tools: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Create with default configuration
    pub fn with_sender(ui_sender: mpsc::UnboundedSender<InteractiveMessage>) -> Self {
        Self::new(InteractiveOutputConfig::default(), ui_sender)
    }
    
    /// Send message to UI
    async fn send_to_ui(&self, message: InteractiveMessage) {
        if let Err(e) = self.ui_sender.send(message) {
            error!("Failed to send message to UI: {}", e);
        }
    }
    
    /// Handle real-time tool execution updates
    async fn handle_tool_update(&self, tool_info: &trae_agent_core::output::ToolExecutionInfo) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.realtime_updates {
            return Ok(());
        }
        
        let mut active_tools = self.active_tools.lock().await;
        
        match tool_info.status {
            ToolExecutionStatus::Executing => {
                // Send executing status to UI
                let status_msg = self.tool_formatter.format_tool_status(tool_info);
                self.send_to_ui(InteractiveMessage::ToolStatus(status_msg)).await;
                active_tools.insert(tool_info.execution_id.clone(), tool_info.clone());
            }
            ToolExecutionStatus::Success | ToolExecutionStatus::Error => {
                // Send final status to UI
                let status_msg = self.tool_formatter.format_tool_status(tool_info);
                self.send_to_ui(InteractiveMessage::ToolStatus(status_msg)).await;
                
                // Send result content if available and configured to show details
                if self.config.show_tool_details {
                    if let Some(result_display) = self.tool_formatter.format_tool_result(tool_info) {
                        self.send_to_ui(InteractiveMessage::ToolResult(result_display)).await;
                    }
                    
                    // Send diff for edit tools
                    if tool_info.tool_name == "str_replace_based_edit_tool" {
                        if let Some(diff_display) = self.diff_formatter.format_edit_result(tool_info) {
                            self.send_to_ui(InteractiveMessage::ToolResult(diff_display)).await;
                        }
                    }
                }
                
                active_tools.remove(&tool_info.execution_id);
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl AgentOutput for InteractiveOutputHandler {
    async fn emit_event(&self, event: AgentEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match event {
            AgentEvent::ExecutionStarted { context } => {
                debug!("🚀 Starting task execution...");
                debug!("📝 Task: {}", context.task);
                debug!("📁 Project path: {}", context.project_path);

                // Send system message to UI
                let msg = format!("⏳ Executing task: {}", context.task);
                self.send_to_ui(InteractiveMessage::SystemMessage(msg)).await;
            }
            
            AgentEvent::ExecutionCompleted { context, success, summary } => {
                // Send completion message
                self.send_to_ui(InteractiveMessage::TaskCompleted { success, summary: summary.clone() }).await;
                
                // Send execution statistics
                let token_msg = if context.token_usage.total_tokens > 0 {
                    Some(format!("🪙 {} input + {} output = {} total tokens",
                        context.token_usage.input_tokens,
                        context.token_usage.output_tokens,
                        context.token_usage.total_tokens))
                } else {
                    None
                };
                
                self.send_to_ui(InteractiveMessage::ExecutionStats {
                    steps: context.current_step,
                    duration: context.execution_time.as_secs_f64(),
                    tokens: token_msg,
                }).await;

                if success {
                    debug!("✅ Task Completed!");
                    debug!("Summary: {}", summary);
                } else {
                    debug!("❌ Task Failed!");
                    debug!("Error: {}", summary);
                }
            }
            
            AgentEvent::StepStarted { step_info } => {
                debug!("🔄 Step {}: {}", step_info.step_number, step_info.task);
            }
            
            AgentEvent::StepCompleted { step_info: _ } => {
                // Usually handled by individual tool completions
            }
            
            AgentEvent::ToolExecutionStarted { tool_info } => {
                // Skip status display for silent tools like sequentialthinking
                if !is_silent_tool(&tool_info.tool_name) {
                    let status_msg = self.tool_formatter.format_tool_status(&tool_info);
                    self.send_to_ui(InteractiveMessage::ToolStatus(status_msg)).await;
                }
                // Always track tools for potential updates
                let mut active_tools = self.active_tools.lock().await;
                active_tools.insert(tool_info.execution_id.clone(), tool_info);
            }

            AgentEvent::ToolExecutionUpdated { tool_info } => {
                self.handle_tool_update(&tool_info).await?;
            }

            AgentEvent::ToolExecutionCompleted { tool_info } => {
                // Skip all output for silent tools - their content is handled separately
                if is_silent_tool(&tool_info.tool_name) {
                    return Ok(());
                }

                let mut active_tools = self.active_tools.lock().await;
                active_tools.remove(&tool_info.execution_id);

                // Send final status to UI
                let status_msg = self.tool_formatter.format_tool_status(&tool_info);
                self.send_to_ui(InteractiveMessage::ToolStatus(status_msg)).await;

                // Send result content if configured to show details
                if self.config.show_tool_details {
                    if let Some(result_display) = self.tool_formatter.format_tool_result(&tool_info) {
                        self.send_to_ui(InteractiveMessage::ToolResult(result_display)).await;
                    }

                    // Send diff for edit tools
                    if tool_info.tool_name == "str_replace_based_edit_tool" {
                        if let Some(diff_display) = self.diff_formatter.format_edit_result(&tool_info) {
                            self.send_to_ui(InteractiveMessage::ToolResult(diff_display)).await;
                        }
                    }
                }
            }
            
            AgentEvent::AgentThinking { step_number: _, thinking } => {
                // Send thinking content to UI
                self.send_to_ui(InteractiveMessage::AgentThinking(thinking)).await;
            }
            
            AgentEvent::Message { level, content, metadata: _ } => {
                let formatted_msg = match level {
                    MessageLevel::Debug => format!("🐛 Debug: {}", content),
                    MessageLevel::Info => format!("ℹ️  {}", content),
                    MessageLevel::Warning => format!("⚠️  Warning: {}", content),
                    MessageLevel::Error => format!("❌ Error: {}", content),
                };
                
                self.send_to_ui(InteractiveMessage::SystemMessage(formatted_msg)).await;
                
                // Also log to tracing
                match level {
                    MessageLevel::Debug => debug!("{}", content),
                    MessageLevel::Info => info!("{}", content),
                    MessageLevel::Warning => warn!("{}", content),
                    MessageLevel::Error => error!("{}", content),
                }
            }
        }
        
        Ok(())
    }
    
    fn supports_realtime_updates(&self) -> bool {
        self.config.realtime_updates
    }
    
    async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // For interactive mode, flushing is handled by the UI
        Ok(())
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
