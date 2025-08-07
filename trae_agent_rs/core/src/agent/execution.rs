//! Agent execution result structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecution {
    /// Whether the execution was successful
    pub success: bool,
    
    /// Final result message
    pub final_result: String,
    
    /// Number of steps executed
    pub steps_executed: usize,
    
    /// Total execution time in milliseconds
    pub duration_ms: u64,
    
    /// Optional structured data
    pub data: Option<serde_json::Value>,
    
    /// Optional metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl AgentExecution {
    /// Create a successful execution result
    pub fn success(final_result: String, steps_executed: usize, duration_ms: u64) -> Self {
        Self {
            success: true,
            final_result,
            steps_executed,
            duration_ms,
            data: None,
            metadata: None,
        }
    }
    
    /// Create a failed execution result
    pub fn failure(error: String, steps_executed: usize, duration_ms: u64) -> Self {
        Self {
            success: false,
            final_result: format!("Execution failed: {}", error),
            steps_executed,
            duration_ms,
            data: None,
            metadata: None,
        }
    }
    
    /// Add structured data to the result
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
    
    /// Add metadata to the result
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}
