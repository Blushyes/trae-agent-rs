//! MCP (Model Context Protocol) tool support

use crate::error::Result;
use crate::tools::{Tool, ToolCall, ToolExample, ToolResult};
use crate::impl_tool_factory;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::{timeout, Duration};

/// MCP server configuration
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub command: Vec<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub timeout_seconds: u64,
}

/// MCP server instance
pub struct McpServer {
    config: McpServerConfig,
    process: Option<Child>,
    request_id: Arc<std::sync::Mutex<u64>>,
    started: bool,
}

impl McpServer {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            process: None,
            request_id: Arc::new(std::sync::Mutex::new(0)),
            started: false,
        }
    }

    /// Start the MCP server process
    pub async fn start(&mut self) -> Result<()> {
        if self.started {
            return Ok(());
        }

        let mut cmd = Command::new(&self.config.command[0]);
        if self.config.command.len() > 1 {
            cmd.args(&self.config.command[1..]);
        }
        cmd.args(&self.config.args);

        // Set environment variables
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        cmd.stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        self.process = Some(cmd.spawn()?);
        self.started = true;

        // Send initialization request
        self.initialize().await?;

        Ok(())
    }

    /// Stop the MCP server
    pub fn stop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        }
        self.started = false;
    }

    /// Send initialization request to MCP server
    async fn initialize(&mut self) -> Result<()> {
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": self.next_request_id(),
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "clientInfo": {
                    "name": "trae-agent-rs",
                    "version": "0.1.0"
                }
            }
        });

        self.send_request(init_request).await?;
        Ok(())
    }

    /// Get next request ID
    fn next_request_id(&self) -> u64 {
        let mut id = self.request_id.lock().unwrap();
        *id += 1;
        *id
    }

    /// Send a JSON-RPC request to the MCP server
    async fn send_request(&mut self, request: Value) -> Result<Value> {
        if !self.started || self.process.is_none() {
            return Err("MCP server not started".into());
        }

        let process = self.process.as_mut().unwrap();
        
        // Send request
        if let Some(stdin) = process.stdin.as_mut() {
            let request_str = serde_json::to_string(&request)?;
            stdin.write_all(request_str.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        } else {
            return Err("No stdin available for MCP server".into());
        }

        // Read response with timeout
        let response = timeout(
            Duration::from_secs(self.config.timeout_seconds),
            self.read_response()
        ).await??;

        Ok(response)
    }

    /// Read JSON-RPC response from MCP server
    async fn read_response(&mut self) -> Result<Value> {
        if let Some(process) = self.process.as_mut() {
            if let Some(stdout) = process.stdout.as_mut() {
                let mut reader = BufReader::new(stdout);
                let mut line = String::new();
                reader.read_line(&mut line).await?;
                
                if line.trim().is_empty() {
                    return Err("Empty response from MCP server".into());
                }

                let response: Value = serde_json::from_str(line.trim())?;
                Ok(response)
            } else {
                Err("No stdout available for MCP server".into())
            }
        } else {
            Err("MCP server process not available".into())
        }
    }

    /// List available tools from MCP server
    pub async fn list_tools(&mut self) -> Result<Vec<Value>> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_request_id(),
            "method": "tools/list"
        });

        let response = self.send_request(request).await?;
        
        if let Some(result) = response.get("result") {
            if let Some(tools) = result.get("tools") {
                if let Some(tools_array) = tools.as_array() {
                    return Ok(tools_array.clone());
                }
            }
        }

        Ok(Vec::new())
    }

    /// Call a tool on the MCP server
    pub async fn call_tool(&mut self, tool_name: &str, arguments: Value) -> Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_request_id(),
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        });

        let response = self.send_request(request).await?;
        
        if let Some(error) = response.get("error") {
            return Err(format!("MCP tool error: {}", error).into());
        }

        if let Some(result) = response.get("result") {
            return Ok(result.clone());
        }

        Err("No result in MCP response".into())
    }
}

/// Tool for interacting with MCP servers
pub struct McpTool {
    servers: Arc<Mutex<HashMap<String, McpServer>>>,
}

impl McpTool {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        "mcp_tool"
    }
    
    fn description(&self) -> &str {
        "Tool for interacting with MCP (Model Context Protocol) servers\n\
         * Manages connections to external MCP servers\n\
         * Provides access to tools exposed by MCP servers\n\
         * Supports server lifecycle management (start, stop, restart)\n\
         * Handles JSON-RPC communication with MCP servers\n\
         \n\
         Operations:\n\
         - `start_server`: Start an MCP server with given configuration\n\
         - `stop_server`: Stop a running MCP server\n\
         - `list_servers`: List all configured MCP servers\n\
         - `list_tools`: List tools available from a specific MCP server\n\
         - `call_tool`: Call a tool on a specific MCP server\n\
         \n\
         MCP servers are external processes that expose tools and resources\n\
         through the Model Context Protocol. This allows integration with\n\
         various external systems and services."
    }
    
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["start_server", "stop_server", "list_servers", "list_tools", "call_tool"],
                    "description": "The operation to perform"
                },
                "server_name": {
                    "type": "string",
                    "description": "Name of the MCP server (required for most operations)"
                },
                "command": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Command to start the MCP server (required for start_server)"
                },
                "args": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Arguments for the MCP server command"
                },
                "env": {
                    "type": "object",
                    "description": "Environment variables for the MCP server"
                },
                "timeout_seconds": {
                    "type": "integer",
                    "description": "Timeout for MCP server operations in seconds (default: 30)"
                },
                "tool_name": {
                    "type": "string",
                    "description": "Name of the tool to call (required for call_tool)"
                },
                "tool_arguments": {
                    "type": "object",
                    "description": "Arguments to pass to the tool (required for call_tool)"
                }
            },
            "required": ["operation"]
        })
    }
    
    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let operation: String = call.get_parameter("operation")?;
        
        match operation.as_str() {
            "start_server" => {
                let server_name: String = call.get_parameter("server_name")?;
                let command: Vec<String> = call.get_parameter("command")?;
                let args: Vec<String> = call.get_parameter_or("args", Vec::new());
                let env: HashMap<String, String> = call.get_parameter_or("env", HashMap::new());
                let timeout_seconds: u64 = call.get_parameter_or("timeout_seconds", 30);
                
                self.start_server(&call.id, server_name, command, args, env, timeout_seconds).await
            }
            "stop_server" => {
                let server_name: String = call.get_parameter("server_name")?;
                self.stop_server(&call.id, server_name).await
            }
            "list_servers" => {
                self.list_servers(&call.id).await
            }
            "list_tools" => {
                let server_name: String = call.get_parameter("server_name")?;
                self.list_tools(&call.id, server_name).await
            }
            "call_tool" => {
                let server_name: String = call.get_parameter("server_name")?;
                let tool_name: String = call.get_parameter("tool_name")?;
                let tool_arguments: Value = call.get_parameter("tool_arguments")?;
                
                self.call_tool(&call.id, server_name, tool_name, tool_arguments).await
            }
            _ => Ok(ToolResult::error(&call.id, &format!(
                "Unknown operation: {}. Supported operations: start_server, stop_server, list_servers, list_tools, call_tool", 
                operation
            ))),
        }
    }
    
    fn examples(&self) -> Vec<ToolExample> {
        vec![
            ToolExample {
                description: "Start an MCP server".to_string(),
                parameters: json!({
                    "operation": "start_server",
                    "server_name": "filesystem",
                    "command": ["node", "/path/to/mcp-server.js"],
                    "args": ["--port", "3000"],
                    "env": {"NODE_ENV": "production"}
                }),
                expected_result: "MCP server started successfully".to_string(),
            },
            ToolExample {
                description: "List tools from an MCP server".to_string(),
                parameters: json!({
                    "operation": "list_tools",
                    "server_name": "filesystem"
                }),
                expected_result: "List of available tools".to_string(),
            },
            ToolExample {
                description: "Call a tool on an MCP server".to_string(),
                parameters: json!({
                    "operation": "call_tool",
                    "server_name": "filesystem",
                    "tool_name": "read_file",
                    "tool_arguments": {"path": "/path/to/file.txt"}
                }),
                expected_result: "Tool execution result".to_string(),
            },
        ]
    }
}

impl McpTool {
    /// Start an MCP server
    async fn start_server(
        &self,
        call_id: &str,
        server_name: String,
        command: Vec<String>,
        args: Vec<String>,
        env: HashMap<String, String>,
        timeout_seconds: u64,
    ) -> Result<ToolResult> {
        if command.is_empty() {
            return Ok(ToolResult::error(call_id, "Command cannot be empty"));
        }

        let config = McpServerConfig {
            name: server_name.clone(),
            command,
            args,
            env,
            timeout_seconds,
        };

        let mut server = McpServer::new(config);

        match server.start().await {
            Ok(()) => {
                let mut servers = self.servers.lock().await;
                servers.insert(server_name.clone(), server);

                Ok(ToolResult::success(call_id, &format!(
                    "MCP server '{}' started successfully", server_name
                )))
            }
            Err(e) => {
                Ok(ToolResult::error(call_id, &format!(
                    "Failed to start MCP server '{}': {}", server_name, e
                )))
            }
        }
    }

    /// Stop an MCP server
    async fn stop_server(&self, call_id: &str, server_name: String) -> Result<ToolResult> {
        let mut servers = self.servers.lock().await;

        if let Some(mut server) = servers.remove(&server_name) {
            server.stop();
            Ok(ToolResult::success(call_id, &format!(
                "MCP server '{}' stopped successfully", server_name
            )))
        } else {
            Ok(ToolResult::error(call_id, &format!(
                "MCP server '{}' not found", server_name
            )))
        }
    }

    /// List all MCP servers
    async fn list_servers(&self, call_id: &str) -> Result<ToolResult> {
        let servers = self.servers.lock().await;

        if servers.is_empty() {
            return Ok(ToolResult::success(call_id, "No MCP servers are currently running"));
        }

        let mut result = String::from("Running MCP servers:\n\n");
        for (name, server) in servers.iter() {
            result.push_str(&format!(
                "- {} (command: {:?}, started: {})\n",
                name,
                server.config.command,
                server.started
            ));
        }

        Ok(ToolResult::success(call_id, &result))
    }

    /// List tools from an MCP server
    async fn list_tools(&self, call_id: &str, server_name: String) -> Result<ToolResult> {
        let mut servers = self.servers.lock().await;

        if let Some(server) = servers.get_mut(&server_name) {
            match server.list_tools().await {
                Ok(tools) => {
                    if tools.is_empty() {
                        Ok(ToolResult::success(call_id, &format!(
                            "No tools available from MCP server '{}'", server_name
                        )))
                    } else {
                        let mut result = format!("Tools available from MCP server '{}':\n\n", server_name);

                        for (i, tool) in tools.iter().enumerate() {
                            if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                                result.push_str(&format!("{}. {}", i + 1, name));

                                if let Some(description) = tool.get("description").and_then(|d| d.as_str()) {
                                    result.push_str(&format!(" - {}", description));
                                }
                                result.push('\n');

                                if let Some(input_schema) = tool.get("inputSchema") {
                                    result.push_str(&format!("   Input schema: {}\n",
                                        serde_json::to_string_pretty(input_schema).unwrap_or_default()));
                                }
                                result.push('\n');
                            }
                        }

                        Ok(ToolResult::success(call_id, &result))
                    }
                }
                Err(e) => {
                    Ok(ToolResult::error(call_id, &format!(
                        "Failed to list tools from MCP server '{}': {}", server_name, e
                    )))
                }
            }
        } else {
            Ok(ToolResult::error(call_id, &format!(
                "MCP server '{}' not found", server_name
            )))
        }
    }

    /// Call a tool on an MCP server
    async fn call_tool(
        &self,
        call_id: &str,
        server_name: String,
        tool_name: String,
        tool_arguments: Value,
    ) -> Result<ToolResult> {
        let mut servers = self.servers.lock().await;

        if let Some(server) = servers.get_mut(&server_name) {
            match server.call_tool(&tool_name, tool_arguments).await {
                Ok(result) => {
                    let result_str = if result.is_string() {
                        result.as_str().unwrap_or("").to_string()
                    } else {
                        serde_json::to_string_pretty(&result).unwrap_or_default()
                    };

                    Ok(ToolResult::success(call_id, &format!(
                        "Tool '{}' executed successfully on MCP server '{}':\n\n{}",
                        tool_name, server_name, result_str
                    )))
                }
                Err(e) => {
                    Ok(ToolResult::error(call_id, &format!(
                        "Failed to call tool '{}' on MCP server '{}': {}",
                        tool_name, server_name, e
                    )))
                }
            }
        } else {
            Ok(ToolResult::error(call_id, &format!(
                "MCP server '{}' not found", server_name
            )))
        }
    }
}

impl_tool_factory!(McpToolFactory, McpTool, "mcp_tool", "Tool for interacting with MCP (Model Context Protocol) servers");
