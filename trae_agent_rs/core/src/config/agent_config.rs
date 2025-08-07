//! Agent configuration structures

use serde::{Deserialize, Serialize};

/// Output mode for the agent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OutputMode {
    /// Debug mode with detailed logging and verbose output
    Debug,
    /// Normal mode with clean, user-friendly output
    Normal,
}

impl Default for OutputMode {
    fn default() -> Self {
        Self::Normal
    }
}

/// Configuration for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Model to use for this agent
    pub model: String,

    /// Maximum number of execution steps
    pub max_steps: usize,

    /// Whether to enable lakeview integration
    pub enable_lakeview: bool,

    /// List of tools available to this agent
    pub tools: Vec<String>,

    /// Output mode for the agent (debug or normal)
    #[serde(default)]
    pub output_mode: OutputMode,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "default_model".to_string(),
            max_steps: 200,
            enable_lakeview: true,
            tools: vec![
                "bash".to_string(),
                "str_replace_based_edit_tool".to_string(),
                "sequentialthinking".to_string(),
                "task_done".to_string(),
            ],
            output_mode: OutputMode::default(),
        }
    }
}
