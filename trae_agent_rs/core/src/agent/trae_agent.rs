//! TraeAgent implementation

use crate::agent::{ Agent, AgentExecution, AgentResult };
use crate::agent::prompt::{ build_system_prompt_with_context, build_user_message };
use crate::config::{ AgentConfig, Config };
use crate::config::agent_config::OutputMode;
use std::io::Write;
use crate::error::{ AgentError, Result };
use crate::llm::{ LlmClient, LlmMessage, LlmResponse, ChatOptions };
use crate::tools::{ ToolExecutor, ToolRegistry };
use crate::trajectory::{ TrajectoryEntry, TrajectoryRecorder };
use async_trait::async_trait;
use futures::StreamExt;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// TraeAgent - the main agent implementation
pub struct TraeAgent {
    config: AgentConfig,
    llm_client: Arc<dyn LlmClient>,
    tool_executor: ToolExecutor,
    trajectory_recorder: Option<TrajectoryRecorder>,
    conversation_history: Vec<LlmMessage>,
    current_task_displayed: bool,
}

impl TraeAgent {
    /// Create a new TraeAgent
    pub async fn new(agent_config: AgentConfig, config: Config) -> Result<Self> {
        // Get model configuration
        let model_config = config
            .get_model(&agent_config.model)
            .ok_or_else(|| AgentError::NotInitialized)?
            .clone();

        // Get provider configuration
        let provider_config = config
            .get_provider(&model_config.model_provider)
            .ok_or_else(|| AgentError::NotInitialized)?
            .clone();

        // Create LLM client
        let llm_client: Arc<dyn LlmClient> = match provider_config.provider.as_str() {
            "anthropic" =>
                Arc::new(crate::llm::AnthropicClient::new(&provider_config, &model_config)?),
            "openai" => Arc::new(crate::llm::OpenAiClient::new(&provider_config, &model_config)?),
            _ => {
                return Err(AgentError::NotInitialized.into());
            }
        };

        // Create tool executor
        let tool_registry = ToolRegistry::default();
        let tool_executor = tool_registry.create_executor(&agent_config.tools);

        Ok(Self {
            config: agent_config,
            llm_client,
            tool_executor,
            trajectory_recorder: None,
            conversation_history: Vec::new(),
            current_task_displayed: false,
        })
    }

    /// Get the system prompt for the agent with project context
    fn get_system_prompt(&self, project_path: &Path) -> String {
        // Use the system prompt with environment context from prompt.rs
        format!(
            "{}\n\nAvailable tools: {}",
            build_system_prompt_with_context(project_path),
            self.tool_executor.list_tools().join(", ")
        )
    }

    /// Execute a step with streaming LLM response
    async fn execute_step_with_streaming(
        &mut self,
        messages: Vec<LlmMessage>,
        tool_definitions: Vec<crate::llm::ToolDefinition>,
        _step: usize
    ) -> Result<LlmResponse> {
        // Set up streaming options
        let options = Some(ChatOptions {
            stream: Some(true),
            ..Default::default()
        });

        // Start streaming
        let mut stream = self.llm_client.chat_completion_stream(
            messages,
            Some(tool_definitions),
            options
        ).await?;

        let mut full_content = String::new();
        let mut final_usage = None;
        let mut final_finish_reason = None;
        let mut tool_call_accumulator: std::collections::HashMap<
            String,
            (String, String)
        > = std::collections::HashMap::new();

        // Only show "Agent thinking..." in debug mode
        if self.config.output_mode == OutputMode::Debug {
            println!("🤖 Agent thinking...");
        }

        // Process stream chunks
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    if let Some(delta) = chunk.delta {
                        print!("{}", delta);
                        full_content.push_str(&delta);
                        // Flush stdout to show text as it arrives
                        use std::io::{ self, Write };
                        io::stdout().flush().unwrap();
                    }

                    // Accumulate tool calls from streaming
                    if let Some(tool_calls) = chunk.tool_calls {
                        for tool_call in tool_calls {
                            let id = tool_call.id.clone();
                            let name = tool_call.name.clone();
                            let params_str = match &tool_call.parameters {
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            };

                            // Accumulate tool call data
                            let entry = tool_call_accumulator
                                .entry(id)
                                .or_insert((String::new(), String::new()));
                            if !name.is_empty() {
                                entry.0 = name; // Update name
                            }
                            entry.1.push_str(&params_str); // Accumulate parameters
                        }
                    }

                    if let Some(usage) = chunk.usage {
                        final_usage = Some(usage);
                    }

                    if let Some(finish_reason) = chunk.finish_reason {
                        final_finish_reason = Some(finish_reason);
                        break;
                    }
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        println!(); // Add newline after streaming

        // Construct the final response with accumulated tool calls
        let mut final_tool_calls = Vec::new();

        for (id, (name, params_str)) in tool_call_accumulator {
            if !name.is_empty() && !params_str.is_empty() {
                // Parse the accumulated parameters as JSON
                match serde_json::from_str(&params_str) {
                    Ok(params) => {
                        final_tool_calls.push(crate::llm::ContentBlock::ToolUse {
                            id,
                            name,
                            input: params,
                        });
                    }
                    Err(_e) => {
                        // Truncate error message for tool params parsing
                        let truncated_params = if params_str.len() > 100 {
                            format!("{}...", &params_str[..100])
                        } else {
                            params_str
                        };
                        println!("🔧 Failed to parse tool params: {}", truncated_params);
                    }
                }
            }
        }

        let message_content = if final_tool_calls.is_empty() {
            crate::llm::MessageContent::Text(full_content)
        } else {
            let mut content_blocks = Vec::new();
            if !full_content.is_empty() {
                content_blocks.push(crate::llm::ContentBlock::Text { text: full_content });
            }
            content_blocks.extend(final_tool_calls);
            crate::llm::MessageContent::MultiModal(content_blocks)
        };

        let response = LlmResponse {
            message: LlmMessage {
                role: crate::llm::MessageRole::Assistant,
                content: message_content,
                metadata: None,
            },
            usage: final_usage,
            model: self.llm_client.model_name().to_string(),
            finish_reason: final_finish_reason,
            metadata: None,
        };

        Ok(response)
    }

    /// Execute a single step of the agent
    async fn execute_step(&mut self, step: usize, project_path: &Path) -> Result<bool> {

        // Prepare messages - only add system prompt if conversation history doesn't start with one
        let mut messages = Vec::new();
        let needs_system_prompt = self.conversation_history.is_empty() ||
            !matches!(self.conversation_history[0].role, crate::llm::MessageRole::System);

        if needs_system_prompt {
            messages.push(LlmMessage::system(self.get_system_prompt(project_path)));
        }
        messages.extend(self.conversation_history.clone());

        // Record LLM request
        if let Some(recorder) = &self.trajectory_recorder {
            recorder.record(
                TrajectoryEntry::llm_request(
                    messages.clone(),
                    self.llm_client.model_name().to_string(),
                    self.llm_client.provider_name().to_string(),
                    step
                )
            ).await?;
        }

        // Get tool definitions
        let tool_definitions = self.tool_executor.get_tool_definitions();

        // Only show "Agent thinking..." in debug mode
        if self.config.output_mode == OutputMode::Debug {
            println!("🤖 Agent thinking...");
        }

        // Set up options
        let options = Some(ChatOptions {
            ..Default::default()
        });

        // Make LLM request (non-streaming)
        let response = self.llm_client.chat_completion(messages, Some(tool_definitions), options).await?;

        // Record LLM response
        if let Some(recorder) = &self.trajectory_recorder {
            recorder.record(
                TrajectoryEntry::llm_response(
                    response.message.clone(),
                    response.usage.clone(),
                    response.finish_reason.as_ref().map(|r| format!("{:?}", r)),
                    step
                )
            ).await?;
        }

        // Add response to conversation history
        self.conversation_history.push(response.message.clone());

        // Check if there are tool calls to execute
        if response.message.has_tool_use() {
            let tool_uses = response.message.get_tool_uses();

            for tool_use in tool_uses {
                if let crate::llm::ContentBlock::ToolUse { id, name, input } = tool_use {
                    // Display tool execution based on output mode
                    match self.config.output_mode {
                        OutputMode::Debug => {
                            // Debug mode: show detailed information
                            println!("🔧 Executing tool: {}", name);
                            println!("📝 Parameters: {}", serde_json::to_string_pretty(&input).unwrap_or_else(|_| format!("{:?}", input)));
                        }
                        OutputMode::Normal => {
                            // Normal mode: show simplified format
                            if name == "bash" {
                                // For bash commands, show executing status
                                use crate::tools::output_formatter::{ToolOutputFormatter, ToolStatus};
                                let formatter = ToolOutputFormatter::new();
                                if let Some(command) = input.get("command").and_then(|v| v.as_str()) {
                                    println!("{}", formatter.format_tool_status("Bash", command, ToolStatus::Executing));
                                } else {
                                    println!("{}", formatter.format_tool_status("Bash", "", ToolStatus::Executing));
                                }
                            } else if name == "sequentialthinking" {
                                // For thinking tool, we'll handle output differently later
                                // Don't show execution message for thinking
                            } else if name == "str_replace_based_edit_tool" {
                                // For edit tool, don't show execution message - we'll show formatted result later
                            } else {
                                // For other tools, show simple execution message
                                println!("🔧 {}", name);
                            }
                        }
                    }

                    let tool_call = crate::tools::ToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        parameters: input.clone(),
                        metadata: None,
                    };

                    // Record tool call
                    if let Some(recorder) = &self.trajectory_recorder {
                        recorder.record(TrajectoryEntry::tool_call(tool_call.clone(), step)).await?;
                    }

                    // Execute tool
                    let tool_result = self.tool_executor.execute(tool_call.clone()).await?;

                    // Display tool result based on output mode
                    match self.config.output_mode {
                        OutputMode::Debug => {
                            // Debug mode: show detailed result
                            let truncated_content = if tool_result.content.len() > 300 {
                                format!("{}...", &tool_result.content[..300])
                            } else {
                                tool_result.content.clone()
                            };
                            println!(
                                "🔧 Tool result: success={}, content={}",
                                tool_result.success,
                                truncated_content
                            );
                        }
                        OutputMode::Normal => {
                            // Normal mode: handle different tools differently
                            if name == "sequentialthinking" {
                                // For thinking tool, extract and display the thought content directly
                                if let Some(data) = &tool_result.data {
                                    if let Some(thought) = data.get("thought") {
                                        if let Some(thought_str) = thought.as_str() {
                                            println!("{}", thought_str);
                                        }
                                    }
                                } else {
                                    // Fallback: try to extract from content
                                    if let Some(start) = tool_result.content.find("Thought: ") {
                                        let thought_start = start + "Thought: ".len();
                                        if let Some(end) = tool_result.content[thought_start..].find("\n\n") {
                                            let thought = &tool_result.content[thought_start..thought_start + end];
                                            println!("{}", thought);
                                        }
                                    }
                                }
                            } else if name == "bash" {
                                // For bash commands, update the status and show output
                                use crate::tools::output_formatter::ToolOutputFormatter;
                                let formatter = ToolOutputFormatter::new();

                                if let Some(command) = tool_call.parameters.get("command").and_then(|v| v.as_str()) {
                                    let formatted = formatter.format_tool_result_with_update("Bash", command, &tool_result.content, tool_result.success);
                                    print!("{}", formatted);
                                    std::io::stdout().flush().unwrap_or(());
                                } else {
                                    let formatted = formatter.format_tool_result_with_update("Bash", "", &tool_result.content, tool_result.success);
                                    print!("{}", formatted);
                                    std::io::stdout().flush().unwrap_or(());
                                }
                            } else if name == "str_replace_based_edit_tool" {
                                // For edit tool, show formatted output based on operation type
                                self.display_edit_tool_result(&tool_call, &tool_result);
                            } else if name == "task_done" {
                                // For task completion, show the summary
                                println!("{}", tool_result.content);
                            } else {
                                // For other tools, show basic result
                                if !tool_result.success {
                                    println!("Error: {}", tool_result.content);
                                }
                            }
                        }
                    }

                    // Record tool result
                    if let Some(recorder) = &self.trajectory_recorder {
                        recorder.record(
                            TrajectoryEntry::tool_result(tool_result.clone(), step)
                        ).await?;
                    }

                    // Check if this is a task completion
                    if name == "task_done" && tool_result.success {
                        return Ok(true); // Task completed
                    }

                    // Add tool result to conversation
                    let result_message = LlmMessage {
                        role: crate::llm::MessageRole::Tool,
                        content: crate::llm::MessageContent::MultiModal(
                            vec![crate::llm::ContentBlock::ToolResult {
                                tool_use_id: id.clone(),
                                is_error: Some(!tool_result.success),
                                content: tool_result.content,
                            }]
                        ),
                        metadata: None,
                    };

                    self.conversation_history.push(result_message);
                }
            }

            // After executing tools, proceed to the next step.
            // Align with Python scheduler: one LLM call per step; tool results are appended,
            // and the next step will let the LLM process those results.
            return Ok(false);
        }

        // If no tool calls, we're done for this step
        Ok(false)
    }

    /// Display formatted output for edit tool results
    fn display_edit_tool_result(&self, tool_call: &crate::tools::ToolCall, tool_result: &crate::tools::ToolResult) {
        use crate::tools::output_formatter::ToolOutputFormatter;

        let formatter = ToolOutputFormatter::new();
        let formatted_output = formatter.format_tool_result(tool_call, tool_result);

        if !formatted_output.is_empty() {
            println!("{}", formatted_output);
        }
    }
}

#[async_trait]
impl Agent for TraeAgent {
    async fn execute_task(&mut self, task: &str) -> AgentResult<AgentExecution> {
        // Use execute_task_with_context with current directory as default
        let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        self.execute_task_with_context(task, &current_dir).await
    }

    fn config(&self) -> &AgentConfig {
        &self.config
    }

    fn agent_type(&self) -> &str {
        "trae_agent"
    }

    fn set_trajectory_recorder(&mut self, recorder: TrajectoryRecorder) {
        self.trajectory_recorder = Some(recorder);
    }

    fn trajectory_recorder(&self) -> Option<&TrajectoryRecorder> {
        self.trajectory_recorder.as_ref()
    }
}

impl TraeAgent {
    /// Execute a task with project context (like Python version)
    pub async fn execute_task_with_context(&mut self, task: &str, project_path: &Path) -> AgentResult<AgentExecution> {
        let start_time = Instant::now();

        // Initialize conversation with system prompt and user message with context
        self.conversation_history.clear();
        // Reset task display flag when starting a new conversation
        self.current_task_displayed = false;

        // Display task only once at the beginning, format based on output mode
        if !self.current_task_displayed {
            match self.config.output_mode {
                OutputMode::Debug => {
                    println!("📋 Task: {}", task);
                }
                OutputMode::Normal => {
                    // In normal mode, just show the task without emoji
                    println!("Task: {}", task);
                }
            }
            self.current_task_displayed = true;
        }

        // Record task start
        if let Some(recorder) = &self.trajectory_recorder {
            recorder.record(
                TrajectoryEntry::task_start(
                    task.to_string(),
                    serde_json::to_value(&self.config).unwrap_or_default()
                )
            ).await?;
        }

        // Add system prompt with tool information and environment context
        self.conversation_history.push(LlmMessage::system(self.get_system_prompt(project_path)));

        // Add user message with task only (environment context is now in system prompt)
        let user_message = build_user_message(task);
        self.conversation_history.push(LlmMessage::user(&user_message));

        let mut step = 0;
        let mut task_completed = false;

        // Execute steps until completion or max steps reached
        while step < self.config.max_steps && !task_completed {
            step += 1;

            match self.execute_step(step, project_path).await {
                Ok(completed) => {
                    task_completed = completed;

                    // Record step completion
                    if let Some(recorder) = &self.trajectory_recorder {
                        recorder.record(
                            TrajectoryEntry::step_complete(
                                format!("Step {} completed", step),
                                true,
                                step
                            )
                        ).await?;
                    }
                }
                Err(e) => {
                    // Record error
                    if let Some(recorder) = &self.trajectory_recorder {
                        recorder.record(
                            TrajectoryEntry::error(
                                e.to_string(),
                                Some(format!("Step {}", step)),
                                step
                            )
                        ).await?;
                    }

                    let duration = start_time.elapsed().as_millis() as u64;
                    return Ok(AgentExecution::failure(
                        format!("Error in step {}: {}", step, e),
                        step,
                        duration
                    ));
                }
            }
        }

        let duration = start_time.elapsed().as_millis() as u64;

        // Record task completion
        if let Some(recorder) = &self.trajectory_recorder {
            recorder.record(
                TrajectoryEntry::task_complete(
                    task_completed,
                    if task_completed {
                        "Task completed successfully".to_string()
                    } else {
                        format!("Task incomplete after {} steps", step)
                    },
                    step,
                    duration
                )
            ).await?;
        }

        if task_completed {
            Ok(AgentExecution::success(
                "Task completed successfully".to_string(),
                step,
                duration
            ))
        } else {
            Ok(AgentExecution::failure(
                format!("Task incomplete after {} steps", step),
                step,
                duration
            ))
        }
    }
}
