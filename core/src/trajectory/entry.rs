//! Trajectory entry structures

use crate::llm::LlmMessage;
use crate::tools::{ToolCall, ToolResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A single entry in the execution trajectory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryEntry {
    /// Unique identifier for this entry
    pub id: String,
    
    /// Timestamp when this entry was created
    pub timestamp: DateTime<Utc>,
    
    /// Type of entry
    pub entry_type: EntryType,
    
    /// Step number in the execution
    pub step: usize,
    
    /// Optional metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Type of trajectory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EntryType {
    /// Task started
    TaskStart {
        task: String,
        agent_config: serde_json::Value,
    },
    
    /// LLM request sent
    LlmRequest {
        messages: Vec<LlmMessage>,
        model: String,
        provider: String,
    },
    
    /// LLM response received
    LlmResponse {
        message: LlmMessage,
        usage: Option<crate::llm::Usage>,
        finish_reason: Option<String>,
    },
    
    /// Tool call initiated
    ToolCall {
        call: ToolCall,
    },
    
    /// Tool result received
    ToolResult {
        result: ToolResult,
    },
    
    /// Agent step completed
    StepComplete {
        step_summary: String,
        success: bool,
    },
    
    /// Task completed
    TaskComplete {
        success: bool,
        final_result: String,
        total_steps: usize,
        duration_ms: u64,
    },
    
    /// Error occurred
    Error {
        error: String,
        context: Option<String>,
    },
    
    /// Custom log entry
    Log {
        level: LogLevel,
        message: String,
        context: Option<HashMap<String, serde_json::Value>>,
    },
}

/// Log level for trajectory entries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl TrajectoryEntry {
    /// Create a new trajectory entry
    pub fn new(entry_type: EntryType, step: usize) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            entry_type,
            step,
            metadata: None,
        }
    }
    
    /// Add metadata to the entry
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }
    
    /// Create a task start entry
    pub fn task_start(task: String, agent_config: serde_json::Value) -> Self {
        Self::new(EntryType::TaskStart { task, agent_config }, 0)
    }
    
    /// Create an LLM request entry
    pub fn llm_request(messages: Vec<LlmMessage>, model: String, provider: String, step: usize) -> Self {
        Self::new(EntryType::LlmRequest { messages, model, provider }, step)
    }
    
    /// Create an LLM response entry
    pub fn llm_response(
        message: LlmMessage,
        usage: Option<crate::llm::Usage>,
        finish_reason: Option<String>,
        step: usize,
    ) -> Self {
        Self::new(EntryType::LlmResponse { message, usage, finish_reason }, step)
    }
    
    /// Create a tool call entry
    pub fn tool_call(call: ToolCall, step: usize) -> Self {
        Self::new(EntryType::ToolCall { call }, step)
    }
    
    /// Create a tool result entry
    pub fn tool_result(result: ToolResult, step: usize) -> Self {
        Self::new(EntryType::ToolResult { result }, step)
    }
    
    /// Create a step complete entry
    pub fn step_complete(step_summary: String, success: bool, step: usize) -> Self {
        Self::new(EntryType::StepComplete { step_summary, success }, step)
    }
    
    /// Create a task complete entry
    pub fn task_complete(
        success: bool,
        final_result: String,
        total_steps: usize,
        duration_ms: u64,
    ) -> Self {
        Self::new(
            EntryType::TaskComplete {
                success,
                final_result,
                total_steps,
                duration_ms,
            },
            total_steps,
        )
    }
    
    /// Create an error entry
    pub fn error(error: String, context: Option<String>, step: usize) -> Self {
        Self::new(EntryType::Error { error, context }, step)
    }
    
    /// Create a log entry
    pub fn log(level: LogLevel, message: String, step: usize) -> Self {
        Self::new(EntryType::Log { level, message, context: None }, step)
    }
    
    /// Create a log entry with context
    pub fn log_with_context(
        level: LogLevel,
        message: String,
        context: HashMap<String, serde_json::Value>,
        step: usize,
    ) -> Self {
        Self::new(EntryType::Log { level, message, context: Some(context) }, step)
    }
}
