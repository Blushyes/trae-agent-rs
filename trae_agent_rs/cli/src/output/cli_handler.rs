//! CLI output handler implementation

use trae_agent_core::output::{
    AgentOutput, AgentEvent, ToolExecutionStatus, MessageLevel
};
use super::formatters::{ToolFormatter, DiffFormatter};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// CLI output configuration
#[derive(Debug, Clone)]
pub struct CliOutputConfig {
    /// Whether to use colors in output
    pub use_colors: bool,
    /// Whether to show debug messages
    pub show_debug: bool,
    /// Whether to show timestamps
    pub show_timestamps: bool,
    /// Whether to support real-time updates
    pub realtime_updates: bool,
}

impl Default for CliOutputConfig {
    fn default() -> Self {
        Self {
            use_colors: true,
            show_debug: false,
            show_timestamps: false,
            realtime_updates: true,
        }
    }
}

/// CLI output handler that formats events for terminal display
pub struct CliOutputHandler {
    config: CliOutputConfig,
    tool_formatter: ToolFormatter,
    diff_formatter: DiffFormatter,
    /// Track active tool executions for real-time updates
    active_tools: Arc<Mutex<HashMap<String, trae_agent_core::output::ToolExecutionInfo>>>,
}

impl CliOutputHandler {
    /// Create a new CLI output handler
    pub fn new(config: CliOutputConfig) -> Self {
        Self {
            config,
            tool_formatter: ToolFormatter::new(),
            diff_formatter: DiffFormatter::new(),
            active_tools: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(CliOutputConfig::default())
    }
    
    /// Handle real-time tool execution updates
    async fn handle_tool_update(&self, tool_info: &trae_agent_core::output::ToolExecutionInfo) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.realtime_updates {
            return Ok(());
        }
        
        let mut active_tools = self.active_tools.lock().await;
        
        match tool_info.status {
            ToolExecutionStatus::Executing => {
                // Show initial executing status
                println!("{}", self.tool_formatter.format_tool_status(tool_info));
                active_tools.insert(tool_info.execution_id.clone(), tool_info.clone());
            }
            ToolExecutionStatus::Success | ToolExecutionStatus::Error => {
                // Update the status line and show result
                if active_tools.contains_key(&tool_info.execution_id) {
                    // Clear current line and move cursor up to overwrite the executing line
                    print!("\x1b[1A\x1b[2K\r");
                    println!("{}", self.tool_formatter.format_tool_status(tool_info));
                    
                    // Show result content if available
                    if let Some(result_display) = self.tool_formatter.format_tool_result(tool_info) {
                        println!("{}", result_display);
                    }
                    
                    // Show diff for edit tools
                    if tool_info.tool_name == "str_replace_based_edit_tool" {
                        if let Some(diff_display) = self.diff_formatter.format_edit_result(tool_info) {
                            println!("{}", diff_display);
                        }
                    }
                    
                    active_tools.remove(&tool_info.execution_id);
                } else {
                    // Tool wasn't tracked, just show the final status
                    println!("{}", self.tool_formatter.format_tool_status(tool_info));
                    if let Some(result_display) = self.tool_formatter.format_tool_result(tool_info) {
                        println!("{}", result_display);
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl AgentOutput for CliOutputHandler {
    async fn emit_event(&self, event: AgentEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match event {
            AgentEvent::ExecutionStarted { context } => {
                println!("🚀 Starting task execution...");
                println!("📝 Task: {}", context.task);
                println!("📁 Project path: {}", context.project_path);
                println!();
                println!("⏳ Executing task...");
                println!("Task: {}", context.task);
            }
            
            AgentEvent::ExecutionCompleted { context, success, summary } => {
                if success {
                    println!("✅ Task Completed!");
                    println!();
                    println!("Summary: {}", summary);
                } else {
                    println!("❌ Task Failed!");
                    println!();
                    println!("Error: {}", summary);
                }
                println!("📈 Executed {} steps", context.current_step);
                println!("⏱️  Duration: {:.2}s", context.execution_time.as_secs_f64());
            }
            
            AgentEvent::StepStarted { step_info } => {
                if self.config.show_debug {
                    println!("🔄 Step {}: {}", step_info.step_number, step_info.task);
                }
            }
            
            AgentEvent::StepCompleted { step_info: _ } => {
                // Usually handled by individual tool completions
            }
            
            AgentEvent::ToolExecutionStarted { tool_info } => {
                self.handle_tool_update(&tool_info).await?;
            }
            
            AgentEvent::ToolExecutionUpdated { tool_info } => {
                self.handle_tool_update(&tool_info).await?;
            }
            
            AgentEvent::ToolExecutionCompleted { tool_info } => {
                self.handle_tool_update(&tool_info).await?;
            }
            
            AgentEvent::AgentThinking { step_number: _, thinking } => {
                if self.config.show_debug {
                    println!("💭 Thinking: {}", thinking);
                } else {
                    // In normal mode, show thinking directly without prefix
                    println!("{}", thinking);
                }
            }
            
            AgentEvent::Message { level, content, metadata: _ } => {
                match level {
                    MessageLevel::Debug if self.config.show_debug => {
                        println!("🐛 Debug: {}", content);
                    }
                    MessageLevel::Info => {
                        println!("ℹ️  {}", content);
                    }
                    MessageLevel::Warning => {
                        println!("⚠️  Warning: {}", content);
                    }
                    MessageLevel::Error => {
                        println!("❌ Error: {}", content);
                    }
                    _ => {} // Skip debug messages if not enabled
                }
            }
        }
        
        Ok(())
    }
    
    fn supports_realtime_updates(&self) -> bool {
        self.config.realtime_updates
    }
    
    async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::io::Write;
        std::io::stdout().flush().map_err(|e| e.into())
    }
}
