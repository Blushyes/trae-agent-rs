//! Task completion tool

use crate::error::Result;
use crate::tools::{ Tool, ToolCall, ToolExample, ToolResult };
use crate::impl_tool_factory;
use async_trait::async_trait;
use serde_json::json;

/// Tool for marking tasks as completed
pub struct TaskDoneTool;

impl TaskDoneTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for TaskDoneTool {
    fn name(&self) -> &str {
        "task_done"
    }

    fn description(&self) -> &str {
        "Mark a task as completed. Use this when you have successfully \
         completed the requested task and want to signal completion."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "summary": {
                    "type": "string",
                    "description": "Summary of what was accomplished"
                },
                "details": {
                    "type": "string",
                    "description": "Optional detailed description of the work done"
                }
            },
            "required": ["summary"]
        })
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let summary: String = call.get_parameter("summary")?;
        let details: Option<String> = call.get_parameter("details").ok();

        let mut result = format!("Summary: {}", summary);

        if let Some(ref details) = details {
            result.push_str(&format!("\n\nDetails:\n{}", details));
        }

        Ok(
            ToolResult::success(&call.id, &result).with_data(
                json!({
            "task_completed": true,
            "summary": summary,
            "details": details
        })
            )
        )
    }

    fn examples(&self) -> Vec<ToolExample> {
        vec![
            ToolExample {
                description: "Mark a simple task as done".to_string(),
                parameters: json!({
                    "summary": "Fixed the bug in the authentication module"
                }),
                expected_result: "Task marked as completed with summary".to_string(),
            },
            ToolExample {
                description: "Mark a complex task as done with details".to_string(),
                parameters: json!({
                    "summary": "Implemented new user registration feature",
                    "details": "Added validation, database schema, API endpoints, and tests"
                }),
                expected_result: "Task marked as completed with summary and details".to_string(),
            }
        ]
    }
}

impl Default for TaskDoneTool {
    fn default() -> Self {
        Self::new()
    }
}

impl_tool_factory!(TaskDoneToolFactory, TaskDoneTool, "task_done", "Mark a task as completed");
