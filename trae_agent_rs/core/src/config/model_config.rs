//! Model configuration structures

use serde::{Deserialize, Serialize};

/// Configuration for a specific model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model provider name
    pub model_provider: String,
    
    /// Model name/identifier
    pub model: String,
    
    /// Maximum tokens for generation
    pub max_tokens: Option<u32>,
    
    /// Temperature for generation (0.0 to 1.0)
    pub temperature: Option<f32>,
    
    /// Top-p sampling parameter
    pub top_p: Option<f32>,
    
    /// Top-k sampling parameter
    pub top_k: Option<u32>,
    
    /// Maximum number of retries for failed requests
    pub max_retries: Option<u32>,
    
    /// Whether to enable parallel tool calls
    pub parallel_tool_calls: Option<bool>,
    
    /// Stop sequences for generation
    pub stop_sequences: Option<Vec<String>>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_provider: "anthropic".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            top_p: Some(1.0),
            top_k: None,
            max_retries: Some(3),
            parallel_tool_calls: Some(true),
            stop_sequences: None,
        }
    }
}
