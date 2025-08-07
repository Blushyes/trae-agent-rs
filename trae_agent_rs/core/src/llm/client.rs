//! LLM client trait and response structures

use crate::error::{ LlmError, Result };
use crate::tools::ToolCall;
use async_trait::async_trait;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;

use super::message::LlmMessage;

/// Trait for LLM clients
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Send a chat completion request
    async fn chat_completion(
        &self,
        messages: Vec<LlmMessage>,
        tools: Option<Vec<ToolDefinition>>,
        options: Option<ChatOptions>
    ) -> Result<LlmResponse>;

    /// Get the model name
    fn model_name(&self) -> &str;

    /// Get the provider name
    fn provider_name(&self) -> &str;

    /// Check if the client supports streaming
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Send a streaming chat completion request
    async fn chat_completion_stream(
        &self,
        _messages: Vec<LlmMessage>,
        _tools: Option<Vec<ToolDefinition>>,
        _options: Option<ChatOptions>
    ) -> Result<Box<dyn futures::Stream<Item = Result<LlmStreamChunk>> + Send + Unpin + '_>> {
        Err(
            (LlmError::InvalidRequest {
                message: "Streaming not supported by this client".to_string(),
            }).into()
        )
    }
}

/// Response from an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// The generated message
    pub message: LlmMessage,

    /// Usage statistics
    pub usage: Option<Usage>,

    /// Model used for generation
    pub model: String,

    /// Finish reason
    pub finish_reason: Option<FinishReason>,

    /// Additional metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Streaming chunk from an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmStreamChunk {
    /// Delta content
    pub delta: Option<String>,

    /// Tool calls in this chunk
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Finish reason if this is the last chunk
    pub finish_reason: Option<FinishReason>,

    /// Usage statistics (usually only in the last chunk)
    pub usage: Option<Usage>,
}

/// Usage statistics for a request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,

    /// Number of tokens in the completion
    pub completion_tokens: u32,

    /// Total number of tokens
    pub total_tokens: u32,
}

/// Reason why generation finished
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Generation completed naturally
    Stop,

    /// Hit the maximum token limit
    Length,

    /// Model decided to call a tool
    ToolCalls,

    /// Content was filtered
    ContentFilter,

    /// Other reason
    Other(String),
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Type of tool (usually "function")
    #[serde(rename = "type")]
    pub tool_type: String,

    /// Function definition
    pub function: FunctionDefinition,
}

/// Function definition for tool calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// Name of the function
    pub name: String,

    /// Description of what the function does
    pub description: String,

    /// JSON schema for the function parameters
    pub parameters: serde_json::Value,
}

/// Options for chat completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatOptions {
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,

    /// Temperature for generation
    pub temperature: Option<f32>,

    /// Top-p sampling parameter
    pub top_p: Option<f32>,

    /// Top-k sampling parameter
    pub top_k: Option<u32>,

    /// Stop sequences
    pub stop: Option<Vec<String>>,

    /// Whether to stream the response
    pub stream: Option<bool>,

    /// Tool choice strategy
    pub tool_choice: Option<ToolChoice>,
}

/// Tool choice strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Let the model decide
    Auto,

    /// Never use tools
    None,

    /// Force use of a specific tool
    Required {
        name: String,
    },
}

impl Default for ChatOptions {
    fn default() -> Self {
        Self {
            max_tokens: Some(131072),
            temperature: Some(0.7),
            top_p: Some(1.0),
            top_k: None,
            stop: None,
            stream: Some(false),
            tool_choice: Some(ToolChoice::Auto),
        }
    }
}
