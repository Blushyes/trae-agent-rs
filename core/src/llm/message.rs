//! LLM message structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a message in an LLM conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    /// Role of the message sender
    pub role: MessageRole,
    
    /// Content of the message
    pub content: MessageContent,
    
    /// Optional metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Role of the message sender
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System message (instructions)
    System,
    
    /// User message (human input)
    User,
    
    /// Assistant message (AI response)
    Assistant,
    
    /// Tool message (tool execution result)
    Tool,
}

/// Content of a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content
    Text(String),
    
    /// Multi-modal content with text and other media
    MultiModal(Vec<ContentBlock>),
}

/// A block of content within a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content
    Text {
        text: String,
    },
    
    /// Image content
    Image {
        /// Image data (base64 encoded)
        data: String,
        /// MIME type of the image
        mime_type: String,
    },
    
    /// Tool use request
    ToolUse {
        /// Unique identifier for this tool use
        id: String,
        /// Name of the tool to use
        name: String,
        /// Input parameters for the tool
        input: serde_json::Value,
    },
    
    /// Tool result
    ToolResult {
        /// ID of the tool use this is a result for
        tool_use_id: String,
        /// Whether the tool execution was successful
        is_error: Option<bool>,
        /// Result content
        content: String,
    },
}

impl LlmMessage {
    /// Create a new system message
    pub fn system<S: Into<String>>(content: S) -> Self {
        Self {
            role: MessageRole::System,
            content: MessageContent::Text(content.into()),
            metadata: None,
        }
    }
    
    /// Create a new user message
    pub fn user<S: Into<String>>(content: S) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Text(content.into()),
            metadata: None,
        }
    }
    
    /// Create a new assistant message
    pub fn assistant<S: Into<String>>(content: S) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text(content.into()),
            metadata: None,
        }
    }
    
    /// Create a new tool message
    pub fn tool<S: Into<String>>(content: S) -> Self {
        Self {
            role: MessageRole::Tool,
            content: MessageContent::Text(content.into()),
            metadata: None,
        }
    }
    
    /// Get the text content of the message
    pub fn get_text(&self) -> Option<String> {
        match &self.content {
            MessageContent::Text(text) => Some(text.clone()),
            MessageContent::MultiModal(blocks) => {
                let mut text_parts = Vec::new();
                for block in blocks {
                    if let ContentBlock::Text { text } = block {
                        text_parts.push(text.clone());
                    }
                }
                if text_parts.is_empty() {
                    None
                } else {
                    Some(text_parts.join("\n"))
                }
            }
        }
    }
    
    /// Check if the message contains tool use
    pub fn has_tool_use(&self) -> bool {
        match &self.content {
            MessageContent::Text(_) => false,
            MessageContent::MultiModal(blocks) => {
                blocks.iter().any(|block| matches!(block, ContentBlock::ToolUse { .. }))
            }
        }
    }
    
    /// Extract tool use blocks from the message
    pub fn get_tool_uses(&self) -> Vec<&ContentBlock> {
        match &self.content {
            MessageContent::Text(_) => Vec::new(),
            MessageContent::MultiModal(blocks) => {
                blocks.iter()
                    .filter(|block| matches!(block, ContentBlock::ToolUse { .. }))
                    .collect()
            }
        }
    }
}

impl From<String> for MessageContent {
    fn from(text: String) -> Self {
        MessageContent::Text(text)
    }
}

impl From<&str> for MessageContent {
    fn from(text: &str) -> Self {
        MessageContent::Text(text.to_string())
    }
}
