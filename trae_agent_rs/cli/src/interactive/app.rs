//! Interactive application using iocraft

use anyhow::Result;
use iocraft::prelude::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use tokio::sync::mpsc;
use trae_agent_core::{Config, agent::TraeAgent};
use crate::output::interactive_handler::InteractiveMessage;
use std::cell::RefCell;

/// Wrap text to fit within specified width, breaking at word boundaries
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();

    for line in text.lines() {
        if line.len() <= max_width {
            lines.push(line.to_string());
        } else {
            let mut current_line = String::new();
            let words: Vec<&str> = line.split_whitespace().collect();

            for word in words {
                // If adding this word would exceed the limit
                if !current_line.is_empty() && current_line.len() + 1 + word.len() > max_width {
                    lines.push(current_line);
                    current_line = word.to_string();
                } else {
                    if !current_line.is_empty() {
                        current_line.push(' ');
                    }
                    current_line.push_str(word);
                }
            }

            if !current_line.is_empty() {
                lines.push(current_line);
            }
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Context for interactive mode
#[derive(Debug, Clone)]
struct InteractiveContext {
    config: Config,
    project_path: PathBuf,
}

/// Thread-local storage for interactive context
thread_local! {
    static INTERACTIVE_CONTEXT: RefCell<Option<InteractiveContext>> = RefCell::new(None);
}

/// Message types for the interactive app
#[derive(Debug, Clone)]
pub enum AppMessage {
    UserInput(String),
    AgentResponse(String),
    SystemMessage(String),
    InteractiveUpdate(InteractiveMessage),
    AgentExecutionStarted,
    AgentExecutionCompleted { success: bool },
    Quit,
}

/// Chat message for display
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Agent,
    System,
}

/// Interactive chat application state
pub struct InteractiveApp {
    messages: Arc<Mutex<VecDeque<ChatMessage>>>,
    input_buffer: String,
    is_processing: bool,
    sender: mpsc::UnboundedSender<AppMessage>,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<AppMessage>>>,
    config: Config,
    project_path: PathBuf,
}

impl InteractiveApp {
    /// Create a new interactive app
    pub fn new(config: Config, project_path: PathBuf) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            messages: Arc::new(Mutex::new(VecDeque::new())),
            input_buffer: String::new(),
            is_processing: false,
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            config,
            project_path,
        }
    }
    
    /// Add a message to the chat
    pub fn add_message(&mut self, role: MessageRole, content: String) {
        let message = ChatMessage {
            role,
            content,
            timestamp: chrono::Utc::now(),
        };
        
        if let Ok(mut messages) = self.messages.lock() {
            messages.push_back(message);
            
            // Keep only the last 100 messages
            if messages.len() > 100 {
                messages.pop_front();
            }
        }
    }
    
    /// Get the message sender
    pub fn sender(&self) -> mpsc::UnboundedSender<AppMessage> {
        self.sender.clone()
    }
    
    /// Process incoming messages
    pub fn process_messages(&mut self) {
        let messages = if let Ok(mut receiver) = self.receiver.try_lock() {
            let mut collected_messages = Vec::new();
            while let Ok(message) = receiver.try_recv() {
                collected_messages.push(message);
            }
            collected_messages
        } else {
            Vec::new()
        };

        for message in messages {
            match message {
                AppMessage::UserInput(input) => {
                    self.add_message(MessageRole::User, input.clone());
                    self.is_processing = true;

                    // Start agent execution asynchronously
                    let sender = self.sender.clone();
                    let config = self.config.clone();
                    let project_path = self.project_path.clone();

                    tokio::spawn(async move {
                        let _ = sender.send(AppMessage::AgentExecutionStarted);

                        match execute_agent_task(input, config, project_path, sender.clone()).await {
                            Ok(_) => {
                                let _ = sender.send(AppMessage::AgentExecutionCompleted { success: true });
                            }
                            Err(e) => {
                                let _ = sender.send(AppMessage::SystemMessage(format!("❌ Error: {}", e)));
                                let _ = sender.send(AppMessage::AgentExecutionCompleted { success: false });
                            }
                        }
                    });
                }
                AppMessage::AgentResponse(response) => {
                    self.add_message(MessageRole::Agent, response);
                }
                AppMessage::SystemMessage(msg) => {
                    self.add_message(MessageRole::System, msg);
                }
                AppMessage::InteractiveUpdate(interactive_msg) => {
                    match interactive_msg {
                        InteractiveMessage::AgentThinking(thinking) => {
                            self.add_message(MessageRole::Agent, thinking);
                        }
                        InteractiveMessage::ToolStatus(status) => {
                            self.add_message(MessageRole::System, status);
                        }
                        InteractiveMessage::ToolResult(result) => {
                            self.add_message(MessageRole::Agent, result);
                        }
                        InteractiveMessage::SystemMessage(msg) => {
                            self.add_message(MessageRole::System, msg);
                        }
                        InteractiveMessage::TaskCompleted { success, summary } => {
                            let status_icon = if success { "✅" } else { "❌" };
                            self.add_message(MessageRole::System, format!("{} Task completed: {}", status_icon, summary));
                        }
                        InteractiveMessage::ExecutionStats { steps, duration, tokens } => {
                            let mut stats = format!("📈 Executed {} steps in {:.2}s", steps, duration);
                            if let Some(token_info) = tokens {
                                stats.push_str(&format!("\n{}", token_info));
                            }
                            self.add_message(MessageRole::System, stats);
                        }
                    }
                }
                AppMessage::AgentExecutionStarted => {
                    self.is_processing = true;
                }
                AppMessage::AgentExecutionCompleted { success: _ } => {
                    self.is_processing = false;
                }
                AppMessage::Quit => {
                    // Handle quit
                }
            }
        }
    }
    
    /// Handle user input
    pub fn handle_input(&mut self, input: String) {
        if input.trim().is_empty() {
            return;
        }
        
        if input.trim() == "exit" || input.trim() == "quit" {
            let _ = self.sender.send(AppMessage::Quit);
            return;
        }
        
        let _ = self.sender.send(AppMessage::UserInput(input));
    }
}



/// Interactive mode using iocraft
pub async fn run_rich_interactive(config: Config, project_path: PathBuf) -> Result<()> {
    println!("🎯 Starting Trae Agent Interactive Mode");

    // Store config and project path in a static context for the UI
    INTERACTIVE_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = Some(InteractiveContext { config, project_path });
    });

    // Run the iocraft-based UI
    tokio::task::spawn_blocking(|| {
        smol::block_on(async {
            element!(TraeApp).render_loop().await
        })
    }).await??;

    Ok(())
}

/// Main entry point for interactive mode
pub async fn run_interactive(config: Config, project_path: PathBuf) -> Result<()> {
    run_rich_interactive(config, project_path).await
}

/// TRAE ASCII Art Logo Component
#[component]
fn TraeLogo(mut _hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // TODO need a beautiful logo!
    let logo = r#"
 ███        
░░░███      
  ░░░███    
    ░░░███  
     ███░   
   ███░     
 ███░       
░░░         
"#;

    element! {
        View {
            Text(
                content: logo,
                color: Color::Rgb { r: 0, g: 255, b: 127 }, // 使用更鲜艳的绿色渐变
                weight: Weight::Bold,
            )
        }
    }
}

/// Main TRAE Interactive Application Component
#[component]
fn TraeApp(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut system = hooks.use_context_mut::<SystemContext>();
    let input_value = hooks.use_state(|| String::new());
    let messages = hooks.use_state(|| Vec::<(String, String)>::new()); // (role, content)
    let is_processing = hooks.use_state(|| false);
    let should_exit = hooks.use_state(|| false);

    // Get interactive context
    let interactive_context = INTERACTIVE_CONTEXT.with(|ctx| ctx.borrow().clone());
    let (config, project_path) = if let Some(ctx) = interactive_context {
        (ctx.config, ctx.project_path)
    } else {
        // Fallback to default values
        (Config::default(), PathBuf::from("."))
    };

    // Handle terminal events
    hooks.use_terminal_events({
        let mut input_value = input_value;
        let mut messages = messages;
        let mut is_processing = is_processing;
        let mut should_exit = should_exit;
        move |event| match event {
            TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
                match code {
                    KeyCode::Char('q') if input_value.read().is_empty() => {
                        should_exit.set(true);
                    }
                    KeyCode::Char(c) => {
                        // Add character to input
                        let mut current_input = input_value.read().clone();
                        current_input.push(c);
                        input_value.set(current_input);
                    }
                    KeyCode::Backspace => {
                        // Remove last character
                        let mut current_input = input_value.read().clone();
                        current_input.pop();
                        input_value.set(current_input);
                    }
                    KeyCode::Enter => {
                        let input = input_value.read().clone();
                        if !input.trim().is_empty() {
                            // Add user message
                            let mut current_messages = messages.read().clone();
                            current_messages.push(("user".to_string(), input.clone()));
                            messages.set(current_messages);

                            // Clear input
                            input_value.set(String::new());

                            // Set processing state
                            is_processing.set(true);

                            // Execute agent task asynchronously
                            let config_clone = config.clone();
                            let project_path_clone = project_path.clone();

                            // Create a dummy channel since execute_agent_task expects it
                            let (ui_sender, _ui_receiver) = mpsc::unbounded_channel();

                            tokio::spawn(async move {
                                match execute_agent_task(input, config_clone, project_path_clone, ui_sender.clone()).await {
                                    Ok(_) => {
                                        // Task completed successfully - CLI output handler already showed the results
                                        is_processing.set(false);
                                    }
                                    Err(e) => {
                                        // Show error in UI
                                        let mut current_messages = messages.read().clone();
                                        current_messages.push(("system".to_string(), format!("❌ Error: {}", e)));
                                        messages.set(current_messages);
                                        is_processing.set(false);
                                    }
                                }
                            });
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    });

    if should_exit.get() {
        system.exit();
    }

    element! {
        View(
            flex_direction: FlexDirection::Column,
            height: 100pct,
            padding: 1,
        ) {
            // Header with TRAE logo
            #(if messages.read().is_empty() {
                Some(element! {
                    View(margin_bottom: 2) {
                        TraeLogo
                    }
                })
            } else {
                None
            })

            // Tips section
            #(if messages.read().is_empty() {
                Some(element! {
                    View(
                        margin_bottom: 2,
                        flex_direction: FlexDirection::Column,
                    ) {
                        View() {
                            Text(
                                content: "Tips for getting started:",
                                color: Color::White,
                            )
                        }
                        View() {
                            Text(
                                content: "1. Ask questions, edit files, or run commands.",
                                color: Color::White,
                            )
                        }
                        View() {
                            Text(
                                content: "2. Be specific for the best results.",
                                color: Color::White,
                            )
                        }
                        View() {
                            Text(
                                content: "3. /help for more information.",
                                color: Color::White,
                            )
                        }
                    }
                })
            } else {
                None
            })

            // Chat messages area - 支持文本换行，防止UI错乱
            View(
                flex_grow: 1.0,
                margin_bottom: 1,
                flex_direction: FlexDirection::Column,
            ) {
                #(messages.read().iter().map(|(role, content)| {
                    // 将长文本按行宽换行，防止自动换行导致UI错乱
                    let wrapped_lines = wrap_text(content, 120); // 使用120字符作为行宽限制

                    if role == "user" {
                        element! {
                            View(
                                width: 100pct,
                                margin_bottom: 1,
                                flex_direction: FlexDirection::Column,
                            ) {
                                #(wrapped_lines.iter().enumerate().map(|(i, line)| {
                                    element! {
                                        View(width: 100pct) {
                                            Text(
                                                content: if i == 0 {
                                                    format!("> {}", line)
                                                } else {
                                                    format!("  {}", line) // 续行缩进
                                                },
                                                color: Color::White,
                                            )
                                        }
                                    }
                                }))
                            }
                        }
                    } else {
                        element! {
                            View(
                                width: 100pct,
                                margin_bottom: 1,
                                flex_direction: FlexDirection::Column,
                            ) {
                                #(wrapped_lines.iter().map(|line| {
                                    element! {
                                        View(width: 100pct) {
                                            Text(
                                                content: line,
                                                color: Color::White,
                                            )
                                        }
                                    }
                                }))
                            }
                        }
                    }
                }))
            }

            // Processing indicator
            #(if is_processing.get() {
                Some(element! {
                    View(margin_bottom: 1) {
                        Text(
                            content: "ℹ Processing...",
                            color: Color::Rgb { r: 100, g: 149, b: 237 }, // 蓝色信息提示
                        )
                    }
                })
            } else {
                None
            })

            // Input area - 简约边框风格，单行高度
            View(
                border_style: BorderStyle::Round,
                border_color: Color::Rgb { r: 100, g: 149, b: 237 }, // 蓝色边框
                padding_left: 1,
                padding_right: 1,
                padding_top: 0,
                padding_bottom: 0,
                margin_bottom: 1,
            ) {
                View(
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                ) {
                    Text(
                        content: "> ",
                        color: Color::Rgb { r: 100, g: 149, b: 237 },
                    )
                    #(if input_value.read().is_empty() {
                        Some(element! {
                            Text(
                                content: "Type your message or @path/to/file",
                                color: Color::DarkGrey,
                            )
                        })
                    } else {
                        Some(element! {
                            Text(
                                content: &input_value.to_string(),
                                color: Color::White,
                            )
                        })
                    })
                }
            }

            // Status bar - 简约风格
            View(
                padding: 1,
            ) {
                Text(
                    content: "~/projects/trae-agent-rs (main*)                       no sandbox (see /docs)                        trae-2.5-pro (100% context left)",
                    color: Color::DarkGrey,
                )
            }
        }
    }
}

/// Execute agent task asynchronously and send updates to UI
async fn execute_agent_task(
    task: String,
    config: Config,
    project_path: PathBuf,
    _ui_sender: mpsc::UnboundedSender<AppMessage>,
) -> Result<()> {
    // Use InteractiveOutputHandler which delegates to CliOutputHandler
    use crate::output::interactive_handler::{InteractiveOutputHandler, InteractiveOutputConfig};

    // Get agent configuration
    let agent_config = config.agents.get("trae_agent").cloned().unwrap_or_default();

    // Create a dummy channel for InteractiveMessage (not used since we delegate to CLI)
    let (interactive_sender, _interactive_receiver) = mpsc::unbounded_channel();

    // Create InteractiveOutputHandler with default config (delegates to CLI)
    let interactive_config = InteractiveOutputConfig {
        realtime_updates: true, // Always enable realtime updates for better UX
        show_tool_details: true,
    };
    let interactive_output = Box::new(InteractiveOutputHandler::new(interactive_config, interactive_sender));

    // Create agent with InteractiveOutputHandler (which delegates to CLI)
    let mut agent = TraeAgent::new_with_output(agent_config, config, interactive_output).await?;

    // Execute the task
    let _execution_result = agent.execute_task_with_context(&task, &project_path).await?;

    Ok(())
}


