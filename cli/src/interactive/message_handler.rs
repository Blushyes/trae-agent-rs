//! Message handling and processing for interactive mode
//!
//! This module handles message conversion, content block identification,
//! and message processing logic for the interactive UI.

use crate::output::interactive_handler::InteractiveMessage;
use rand::seq::SliceRandom;

/// Random status words for initial display
const RANDOM_STATUS_WORDS: &[&str] = &[
    "Vicing",
    "Working",
    "Mulling",
    "Unravelling",
    "Finagling",
    "Doing",
    "Brewing",
    "Pondering",
    "Crafting",
    "Weaving",
    "Conjuring",
    "Orchestrating",
    "Assembling",
    "Synthesizing",
    "Formulating",
    "Devising",
    "Constructing",
    "Architecting",
    "Engineering",
    "Designing",
    "Plotting",
    "Scheming",
    "Strategizing",
    "Calculating",
    "Computing",
    "Processing",
    "Analyzing",
    "Examining",
    "Investigating",
    "Exploring",
    "Discovering",
    "Uncovering",
    "Deciphering",
    "Solving",
    "Resolving",
    "Tackling",
    "Addressing",
    "Handling",
    "Managing",
    "Coordinating",
];

/// Represents different types of content blocks for output formatting
#[derive(Debug, Clone, PartialEq)]
pub enum ContentBlock {
    /// User input messages
    UserInput,
    /// Agent thinking, analysis, or response text
    AgentText,
    /// Tool execution status (e.g., "⏺ Read(filename)")
    ToolStatus,
    /// Tool execution results/output
    ToolResult,
}

/// Message types for the interactive app
#[derive(Debug, Clone)]
pub enum AppMessage {
    SystemMessage(String),
    UserMessage(String),
    InteractiveUpdate(InteractiveMessage),
    AgentTaskStarted { operation: String },
    AgentExecutionCompleted,
    AgentExecutionInterrupted { user_input: String },
    TokenUpdate { tokens: u32 },
}

/// Get a random status word
pub fn get_random_status_word() -> String {
    let mut rng = rand::thread_rng();
    let word = RANDOM_STATUS_WORDS.choose(&mut rng).unwrap_or(&"Working");
    format!("{}…", word)
}

/// Generate a unique message ID
pub fn generate_message_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("msg_{}", timestamp)
}

/// Identify the content block type based on content and role
pub fn identify_content_block(content: &str, role: &str) -> ContentBlock {
    if role == "user" {
        return ContentBlock::UserInput;
    }

    // Check for tool status indicators
    if content.contains("⏺") {
        return ContentBlock::ToolStatus;
    }

    // Check for tool result indicators
    if content.contains("⎿") {
        return ContentBlock::ToolResult;
    }

    // Check for bash output patterns (file listings, command output, etc.)
    let is_bash_output = content.starts_with("total ") || // ls -la output
        content.contains("drwx") ||      // directory listing
        content.contains("-rw-") ||      // file listing
        content.lines().any(|line| line.trim().starts_with("total ")) ||
        content.lines().any(|line| line.contains("drwx") || line.contains("-rw-"));

    if is_bash_output {
        return ContentBlock::ToolResult;
    }

    // Default to agent text for everything else
    ContentBlock::AgentText
}

/// Check if content is bash output that should be displayed in gray
pub fn is_bash_output_content(content: &str) -> bool {
    !content.contains("⏺")
        && !content.contains("⎿")
        && (content.starts_with("total ")
            || content.contains("drwx")
            || content.contains("-rw-")
            || content
                .lines()
                .any(|line| line.trim().starts_with("total "))
            || content
                .lines()
                .any(|line| line.contains("drwx") || line.contains("-rw-")))
}

/// Convert AppMessage to UI message tuple (role, content, message_id, is_tool_result)
pub fn app_message_to_ui_message(
    app_message: AppMessage,
) -> Option<(String, String, Option<String>, bool)> {
    match app_message {
        AppMessage::SystemMessage(msg) => Some((
            "system".to_string(),
            msg,
            Some(generate_message_id()),
            false,
        )),
        AppMessage::UserMessage(msg) => {
            Some(("user".to_string(), msg, Some(generate_message_id()), false))
        }
        AppMessage::InteractiveUpdate(interactive_msg) => match interactive_msg {
            InteractiveMessage::AgentThinking(thinking) => Some((
                "agent".to_string(),
                thinking,
                Some(generate_message_id()),
                false,
            )),
            InteractiveMessage::ToolStatus {
                execution_id,
                status,
            } => Some(("system".to_string(), status, Some(execution_id), false)),
            InteractiveMessage::ToolResult(result) => {
                // Use block system to determine if this is bash output
                let is_bash_output_content = is_bash_output_content(&result);

                Some((
                    "agent".to_string(),
                    result,
                    Some(generate_message_id()),
                    is_bash_output_content,
                ))
            }
            InteractiveMessage::SystemMessage(msg) => Some((
                "system".to_string(),
                msg,
                Some(generate_message_id()),
                false,
            )),
            InteractiveMessage::TaskCompleted { success, summary } => {
                let status_icon = if success { "✅" } else { "❌" };
                Some((
                    "system".to_string(),
                    format!("{} Task completed: {}", status_icon, summary),
                    Some(generate_message_id()),
                    false,
                ))
            }
            InteractiveMessage::ExecutionStats {
                steps,
                duration,
                tokens,
            } => {
                let mut stats = format!("📈 Executed {} steps in {:.2}s", steps, duration);
                if let Some(token_info) = tokens {
                    stats.push_str(&format!("\n{}", token_info));
                }
                Some((
                    "system".to_string(),
                    stats,
                    Some(generate_message_id()),
                    false,
                ))
            }
        },
        AppMessage::AgentTaskStarted { .. } => None,
        AppMessage::AgentExecutionCompleted => None,
        AppMessage::AgentExecutionInterrupted { user_input: _ } => Some((
            "system".to_string(),
            "  \x1b[31m⏹ Interrupted by user\x1b[0m".to_string(),
            Some(generate_message_id()),
            false,
        )),
        AppMessage::TokenUpdate { .. } => None, // Token updates don't create UI messages, they update state directly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_random_status_word() {
        let word = get_random_status_word();
        assert!(word.ends_with("…"));
        assert!(!word.is_empty());
    }

    #[test]
    fn test_generate_message_id() {
        let id1 = generate_message_id();
        let id2 = generate_message_id();
        assert!(id1.starts_with("msg_"));
        assert!(id2.starts_with("msg_"));
        assert_ne!(id1, id2); // Should be unique
    }

    #[test]
    fn test_identify_content_block() {
        assert_eq!(
            identify_content_block("Hello", "user"),
            ContentBlock::UserInput
        );
        assert_eq!(
            identify_content_block("⏺ Running command", "agent"),
            ContentBlock::ToolStatus
        );
        assert_eq!(
            identify_content_block("⎿ Result", "agent"),
            ContentBlock::ToolResult
        );
        assert_eq!(
            identify_content_block("total 10", "agent"),
            ContentBlock::ToolResult
        );
        assert_eq!(
            identify_content_block("Regular text", "agent"),
            ContentBlock::AgentText
        );
    }

    #[test]
    fn test_is_bash_output_content() {
        assert!(is_bash_output_content("total 10"));
        assert!(is_bash_output_content("drwxr-xr-x"));
        assert!(is_bash_output_content("-rw-r--r--"));
        assert!(!is_bash_output_content("⏺ Running"));
        assert!(!is_bash_output_content("Regular text"));
    }
}
