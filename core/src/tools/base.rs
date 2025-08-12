//! Base tool traits and structures

use crate::error::{Result, ToolError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Trait for all tools
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the name of the tool
    fn name(&self) -> &str;
    
    /// Get the description of the tool
    fn description(&self) -> &str;
    
    /// Get the JSON schema for the tool's parameters
    fn parameters_schema(&self) -> serde_json::Value;
    
    /// Execute the tool with the given parameters
    async fn execute(&self, call: ToolCall) -> Result<ToolResult>;
    
    /// Check if the tool requires special permissions
    fn requires_confirmation(&self) -> bool {
        false
    }
    
    /// Get examples of how to use this tool
    fn examples(&self) -> Vec<ToolExample> {
        Vec::new()
    }
}

/// A call to a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    
    /// Name of the tool to call
    pub name: String,
    
    /// Parameters to pass to the tool
    pub parameters: serde_json::Value,
    
    /// Optional metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID of the tool call this is a result for
    pub tool_call_id: String,
    
    /// Whether the execution was successful
    pub success: bool,
    
    /// Result content
    pub content: String,
    
    /// Optional structured data
    pub data: Option<serde_json::Value>,
    
    /// Execution duration in milliseconds
    pub duration_ms: Option<u64>,
    
    /// Optional metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Example usage of a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    /// Description of what this example does
    pub description: String,
    
    /// Example parameters
    pub parameters: serde_json::Value,
    
    /// Expected result description
    pub expected_result: String,
}

/// Tool executor that manages tool execution
pub struct ToolExecutor {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolCall {
    /// Create a new tool call
    pub fn new<S: Into<String>>(name: S, parameters: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            parameters,
            metadata: None,
        }
    }
    
    /// Get a parameter value by key
    pub fn get_parameter<T>(&self, key: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let value = self.parameters
            .get(key)
            .ok_or_else(|| ToolError::InvalidParameters {
                message: format!("Missing parameter: {}", key),
            })?;

        serde_json::from_value(value.clone())
            .map_err(|_| ToolError::InvalidParameters {
                message: format!("Invalid parameter type for: {}", key),
            }.into())
    }
    
    /// Get a parameter value by key with a default
    pub fn get_parameter_or<T>(&self, key: &str, default: T) -> T
    where
        T: for<'de> Deserialize<'de> + Clone,
    {
        self.get_parameter(key).unwrap_or(default)
    }
}

impl ToolResult {
    /// Create a successful result
    pub fn success<S: Into<String>>(tool_call_id: S, content: S) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            success: true,
            content: content.into(),
            data: None,
            duration_ms: None,
            metadata: None,
        }
    }
    
    /// Create an error result
    pub fn error<S: Into<String>>(tool_call_id: S, error: S) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            success: false,
            content: format!("Error: {}", error.into()),
            data: None,
            duration_ms: None,
            metadata: None,
        }
    }
    
    /// Set structured data
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
    
    /// Set execution duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }
    
    /// Set metadata
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

impl ToolExecutor {
    /// Create a new tool executor
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    
    /// Register a tool
    pub fn register_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }
    
    /// Get a tool by name
    pub fn get_tool(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }
    
    /// List all available tools
    pub fn list_tools(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
    
    /// Execute a tool call
    pub async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let tool = self.get_tool(&call.name)
            .ok_or_else(|| ToolError::NotFound {
                name: call.name.clone(),
            })?;
        
        let start_time = std::time::Instant::now();
        let call_id = call.id.clone();
        let result = tool.execute(call).await;
        let duration = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(mut result) => {
                result.duration_ms = Some(duration);
                Ok(result)
            }
            Err(e) => Ok(ToolResult::error(&call_id, &e.to_string()).with_duration(duration)),
        }
    }
    
    /// Get tool definitions for LLM function calling
    pub fn get_tool_definitions(&self) -> Vec<crate::llm::ToolDefinition> {
        self.tools
            .values()
            .map(|tool| crate::llm::ToolDefinition {
                tool_type: "function".to_string(),
                function: crate::llm::FunctionDefinition {
                    name: tool.name().to_string(),
                    description: tool.description().to_string(),
                    parameters: tool.parameters_schema(),
                },
            })
            .collect()
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}
