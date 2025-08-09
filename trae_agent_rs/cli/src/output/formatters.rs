//! Formatting utilities for CLI output

use trae_agent_core::output::{ToolExecutionInfo, ToolExecutionStatus};
use std::path::Path;

// ANSI color codes
const RED_BG: &str = "\x1b[41m";    // Red background
const GREEN_BG: &str = "\x1b[42m";  // Green background
const GRAY: &str = "\x1b[90m";      // Gray text for line numbers
const WHITE: &str = "\x1b[97m";     // White text for executing status
const GREEN: &str = "\x1b[92m";     // Green text for success status
const RED: &str = "\x1b[91m";       // Red text for error status
const RESET: &str = "\x1b[0m";

/// Tool execution formatter
pub struct ToolFormatter;

impl ToolFormatter {
    pub fn new() -> Self {
        Self
    }
    
    /// Format tool execution status for CLI display
    pub fn format_tool_status(&self, tool_info: &ToolExecutionInfo) -> String {
        let (dot_color, dot_char) = match tool_info.status {
            ToolExecutionStatus::Executing => (WHITE, "⏺"),
            ToolExecutionStatus::Success => (GREEN, "⏺"),
            ToolExecutionStatus::Error => (RED, "⏺"),
        };
        
        // Extract command or main parameter for display
        let command = self.extract_tool_command(tool_info);
        
        format!("{}{}{} {}({})", dot_color, dot_char, RESET, tool_info.tool_name, command)
    }
    
    /// Extract the main command/parameter from tool info for display
    fn extract_tool_command(&self, tool_info: &ToolExecutionInfo) -> String {
        match tool_info.tool_name.as_str() {
            "bash" => {
                tool_info.parameters
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            }
            "str_replace_based_edit_tool" => {
                tool_info.parameters
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(|path| {
                        Path::new(path)
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or(path)
                            .to_string()
                    })
                    .unwrap_or_else(|| "file".to_string())
            }
            _ => {
                // For other tools, try to find a reasonable display parameter
                tool_info.parameters
                    .get("path")
                    .or_else(|| tool_info.parameters.get("file"))
                    .or_else(|| tool_info.parameters.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            }
        }
    }
    
    /// Format tool result content for display
    pub fn format_tool_result(&self, tool_info: &ToolExecutionInfo) -> Option<String> {
        let result = tool_info.result.as_ref()?;
        
        match tool_info.tool_name.as_str() {
            "bash" => {
                if !result.content.trim().is_empty() {
                    let display_content = if result.content.len() > 200 {
                        format!("{}...", &result.content[..197])
                    } else {
                        result.content.clone()
                    };
                    Some(format!("  ⎿  {}", display_content))
                } else {
                    None
                }
            }
            "str_replace_based_edit_tool" => {
                // Simple success/error message - detailed diff is handled separately
                if result.success {
                    Some("  ⎿  File updated successfully".to_string())
                } else {
                    Some(format!("  ⎿  Error: {}", result.content))
                }
            }
            "task_done" => {
                Some(format!("  ⎿  {}", result.content))
            }
            _ => {
                if !result.content.trim().is_empty() {
                    Some(format!("  ⎿  {}", result.content))
                } else {
                    None
                }
            }
        }
    }
}

/// Diff formatter for file editing operations
pub struct DiffFormatter;

impl DiffFormatter {
    pub fn new() -> Self {
        Self
    }
    
    /// Format edit tool result with diff view
    pub fn format_edit_result(&self, tool_info: &ToolExecutionInfo) -> Option<String> {
        let result = tool_info.result.as_ref()?;
        
        if !result.success {
            return None;
        }
        
        let path = tool_info.parameters
            .get("path")
            .and_then(|v| v.as_str())?;
        
        let file_name = Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(path);
        
        // Determine operation type and create appropriate diff
        if let Some(old_str) = tool_info.parameters.get("old_str").and_then(|v| v.as_str()) {
            if let Some(new_str) = tool_info.parameters.get("new_str").and_then(|v| v.as_str()) {
                // str_replace operation
                Some(self.create_unified_diff_view(file_name, Some(old_str), Some(new_str)))
            } else {
                None
            }
        } else if let Some(new_str) = tool_info.parameters.get("new_str").and_then(|v| v.as_str()) {
            // insert operation
            Some(self.create_unified_diff_view(file_name, None, Some(new_str)))
        } else if let Some(file_text) = tool_info.parameters.get("file_text").and_then(|v| v.as_str()) {
            // create operation
            Some(self.create_unified_diff_view(file_name, None, Some(file_text)))
        } else {
            None
        }
    }
    
    /// Create a unified diff view for all operations
    fn create_unified_diff_view(&self, file_name: &str, old_content: Option<&str>, new_content: Option<&str>) -> String {
        let mut result = String::new();
        
        result.push_str(&format!("╭{:─<120}╮\n", ""));
        result.push_str(&format!("│ {:<118} │\n", file_name));
        result.push_str(&format!("│{:<120}│\n", ""));

        let old_lines: Vec<&str> = old_content.map(|s| s.lines().collect()).unwrap_or_default();
        let new_lines: Vec<&str> = new_content.map(|s| s.lines().collect()).unwrap_or_default();

        let max_lines = old_lines.len().max(new_lines.len());
        
        for i in 0..max_lines {
            let line_num = format!("{:>3}", i + 1);
            
            if i < old_lines.len() && i < new_lines.len() {
                if old_lines[i] != new_lines[i] {
                    // Changed line - show both old and new
                    let old_line = self.format_line_with_background_and_prefix(old_lines[i], RED_BG, "-");
                    let new_line = self.format_line_with_background_and_prefix(new_lines[i], GREEN_BG, "+");
                    result.push_str(&format!("│   {}{}{} {} │\n", GRAY, line_num, RESET, old_line));
                    result.push_str(&format!("│   {}{}{} {} │\n", GRAY, line_num, RESET, new_line));
                } else {
                    // Unchanged line
                    let line = self.truncate_line(old_lines[i]);
                    result.push_str(&format!("│   {}{}{}    {:<100} │\n", GRAY, line_num, RESET, line));
                }
            } else if i < old_lines.len() {
                // Deleted line
                let line = self.format_line_with_background_and_prefix(old_lines[i], RED_BG, "-");
                result.push_str(&format!("│   {}{}{} {} │\n", GRAY, line_num, RESET, line));
            } else if i < new_lines.len() {
                // Added line
                let line = self.format_line_with_background_and_prefix(new_lines[i], GREEN_BG, "+");
                result.push_str(&format!("│   {}{}{} {} │\n", GRAY, line_num, RESET, line));
            }
        }

        result.push_str(&format!("╰{:─<120}╯", ""));
        
        result
    }
    
    /// Format a line with background color including prefix symbol
    fn format_line_with_background_and_prefix(&self, line: &str, bg_color: &str, prefix: &str) -> String {
        let truncated = self.truncate_line(line);
        let content_with_prefix = format!("{} {}", prefix, truncated);
        format!("{}{:<100}{}", bg_color, content_with_prefix, RESET)
    }
    
    /// Truncate line if too long
    fn truncate_line(&self, line: &str) -> String {
        if line.len() > 100 { 
            format!("{}...", &line[..97])
        } else { 
            line.to_string()
        }
    }
}
