//! Task execution module for interactive mode
//! 
//! This module handles agent task execution with UI integration,
//! including token tracking and status updates.

use crate::interactive::message_handler::AppMessage;
use crate::output::interactive_handler::{InteractiveMessage, InteractiveOutputConfig};
use anyhow::Result;
use std::path::PathBuf;
use tokio::sync::{broadcast, mpsc};
use trae_agent_core::Config;

/// Custom output handler that forwards events and tracks tokens
pub struct TokenTrackingOutputHandler {
    interactive_handler: crate::output::interactive_handler::InteractiveOutputHandler,
    ui_sender: broadcast::Sender<AppMessage>,
}

impl TokenTrackingOutputHandler {
    pub fn new(
        interactive_config: InteractiveOutputConfig,
        interactive_sender: mpsc::UnboundedSender<InteractiveMessage>,
        ui_sender: broadcast::Sender<AppMessage>,
    ) -> Self {
        Self {
            interactive_handler: crate::output::interactive_handler::InteractiveOutputHandler::new(
                interactive_config,
                interactive_sender,
            ),
            ui_sender,
        }
    }
}

#[async_trait::async_trait]
impl trae_agent_core::output::AgentOutput for TokenTrackingOutputHandler {
    async fn emit_event(
        &self,
        event: trae_agent_core::output::AgentEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check for token updates and status updates in various events
        match &event {
            trae_agent_core::output::AgentEvent::ExecutionCompleted { context, .. } => {
                if context.token_usage.total_tokens > 0 {
                    let _ = self.ui_sender.send(AppMessage::TokenUpdate {
                        tokens: context.token_usage.total_tokens,
                    });
                }
            }
            trae_agent_core::output::AgentEvent::TokenUsageUpdated { token_usage } => {
                // Send immediate token update for smooth animation
                let _ = self.ui_sender.send(AppMessage::TokenUpdate {
                    tokens: token_usage.total_tokens,
                });
            }
            trae_agent_core::output::AgentEvent::StatusUpdate { status, .. } => {
                // Send status update to UI
                let _ = self.ui_sender.send(AppMessage::AgentTaskStarted {
                    operation: status.clone(),
                });
            }
            _ => {}
        }

        // Forward to the interactive handler
        self.interactive_handler.emit_event(event).await
    }

    fn supports_realtime_updates(&self) -> bool {
        self.interactive_handler.supports_realtime_updates()
    }

    async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.interactive_handler.flush().await
    }
}

/// Execute agent task asynchronously and send updates to UI
pub async fn execute_agent_task(
    task: String,
    config: Config,
    project_path: PathBuf,
    ui_sender: broadcast::Sender<AppMessage>,
) -> Result<()> {
    use crate::tools::StatusReportToolFactory;
    use trae_agent_core::tools::ToolRegistry;

    // Get agent configuration
    let mut agent_config = config.agents.get("trae_agent").cloned().unwrap_or_default();

    // Add status_report tool to the tool list for interactive mode
    if !agent_config.tools.contains(&"status_report".to_string()) {
        agent_config.tools.push("status_report".to_string());
    }

    // Create channel for InteractiveMessage and forward to AppMessage
    let (interactive_sender, mut interactive_receiver) = mpsc::unbounded_channel();
    let ui_sender_clone = ui_sender.clone();

    // Forward InteractiveMessage to AppMessage
    tokio::spawn(async move {
        while let Some(interactive_msg) = interactive_receiver.recv().await {
            let _ = ui_sender_clone.send(AppMessage::InteractiveUpdate(interactive_msg));
        }
    });

    // Create TokenTrackingOutputHandler with UI integration
    let interactive_config = InteractiveOutputConfig {
        realtime_updates: true,
        show_tool_details: true,
    };
    let token_tracking_output = Box::new(TokenTrackingOutputHandler::new(
        interactive_config,
        interactive_sender,
        ui_sender.clone(),
    ));

    // Create custom tool registry with status_report tool for interactive mode
    let mut tool_registry = ToolRegistry::default();
    tool_registry.register_factory(Box::new(StatusReportToolFactory::with_ui_sender(
        ui_sender.clone(),
    )));

    // Create modified config with custom tool registry
    let modified_config = config.clone();

    // Create and execute agent task
    let mut agent = trae_agent_core::agent::TraeAgent::new_with_output_and_registry(
        agent_config,
        modified_config,
        token_tracking_output,
        tool_registry,
    )
    .await?;
    agent
        .execute_task_with_context(&task, &project_path)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;

    #[test]
    fn test_token_tracking_output_handler_creation() {
        let (ui_sender, _) = broadcast::channel::<AppMessage>(10);
        let (interactive_sender, _) = mpsc::unbounded_channel();
        let config = InteractiveOutputConfig {
            realtime_updates: true,
            show_tool_details: true,
        };
        
        let handler = TokenTrackingOutputHandler::new(config, interactive_sender, ui_sender);
        assert!(handler.supports_realtime_updates());
    }
}
