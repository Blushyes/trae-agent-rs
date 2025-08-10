//! Interactive application using iocraft

use anyhow::Result;
use iocraft::prelude::*;
use std::path::PathBuf;
use tokio::sync::mpsc;
use trae_agent_core::{Config, agent::TraeAgent};
use crate::output::interactive_handler::InteractiveMessage;
use std::sync::OnceLock;
use unicode_width::UnicodeWidthStr;
/// Get terminal width with fallback
fn get_terminal_width() -> usize {
    match crossterm::terminal::size() {
        Ok((cols, _)) => {
            // Reserve some space for padding and borders, and ensure minimum width
            let usable_width = (cols as usize).saturating_sub(8); // 8 chars for padding/borders
            std::cmp::max(usable_width, 40) // Minimum 40 chars
        }
        Err(_) => 80, // Fallback to 80 columns
    }
}

/// Wrap text to fit within specified width, breaking at word boundaries
/// Uses unicode-aware width calculation for proper handling of CJK characters
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    for line in text.lines() {
        let line_width = UnicodeWidthStr::width(line);
        if line_width <= max_width {
            lines.push(line.to_string());
        } else {
            // For very long lines, we need to break them more aggressively
            let mut current_line = String::new();
            let mut current_width = 0;

            // First try word-based wrapping
            let words: Vec<&str> = line.split_whitespace().collect();

            for word in words {
                let word_width = UnicodeWidthStr::width(word);

                // If the word itself is too long, we'll need character-based wrapping
                if word_width > max_width {
                    // Push current line if it has content
                    if !current_line.is_empty() {
                        lines.push(current_line);
                        current_line = String::new();
                        current_width = 0;
                    }

                    // Character-based wrapping for very long words
                    let mut char_line = String::new();
                    let mut char_width = 0;

                    for ch in word.chars() {
                        let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());
                        if char_width + ch_width > max_width && !char_line.is_empty() {
                            lines.push(char_line);
                            char_line = ch.to_string();
                            char_width = ch_width;
                        } else {
                            char_line.push(ch);
                            char_width += ch_width;
                        }
                    }

                    if !char_line.is_empty() {
                        current_line = char_line;
                        current_width = char_width;
                    }
                } else {
                    // Normal word wrapping
                    if current_width > 0 && current_width + 1 + word_width > max_width {
                        lines.push(current_line);
                        current_line = word.to_string();
                        current_width = word_width;
                    } else {
                        if current_width > 0 {
                            current_line.push(' ');
                            current_width += 1;
                        }
                        current_line.push_str(word);
                        current_width += word_width;
                    }
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

/// Global storage for interactive context accessible across threads
static INTERACTIVE_CONTEXT: OnceLock<InteractiveContext> = OnceLock::new();

/// Message types for the interactive app
#[derive(Debug, Clone)]
pub enum AppMessage {
    SystemMessage(String),
    InteractiveUpdate(InteractiveMessage),
    AgentExecutionCompleted,
}



/// Get interactive context with fallback to defaults
fn get_interactive_context() -> (Config, PathBuf) {
    if let Some(ctx) = INTERACTIVE_CONTEXT.get() {
        (ctx.config.clone(), ctx.project_path.clone())
    } else {
        (Config::default(), PathBuf::from("."))
    }
}

/// Convert AppMessage to UI message tuple (role, content)
fn app_message_to_ui_message(app_message: AppMessage) -> Option<(String, String)> {
    match app_message {
        AppMessage::SystemMessage(msg) => Some(("system".to_string(), msg)),
        AppMessage::InteractiveUpdate(interactive_msg) => {
            match interactive_msg {
                InteractiveMessage::AgentThinking(thinking) => Some(("agent".to_string(), thinking)),
                InteractiveMessage::ToolStatus(status) => Some(("system".to_string(), status)),
                InteractiveMessage::ToolResult(result) => Some(("agent".to_string(), result)),
                InteractiveMessage::SystemMessage(msg) => Some(("system".to_string(), msg)),
                InteractiveMessage::TaskCompleted { success, summary } => {
                    let status_icon = if success { "✅" } else { "❌" };
                    Some(("system".to_string(), format!("{} Task completed: {}", status_icon, summary)))
                }
                InteractiveMessage::ExecutionStats { steps, duration, tokens } => {
                    let mut stats = format!("📈 Executed {} steps in {:.2}s", steps, duration);
                    if let Some(token_info) = tokens {
                        stats.push_str(&format!("\n{}", token_info));
                    }
                    Some(("system".to_string(), stats))
                }
            }
        }
        AppMessage::AgentExecutionCompleted => None,
    }
}

/// Spawn agent task execution for UI components
fn spawn_ui_agent_task(
    input: String,
    config: Config,
    project_path: PathBuf,
    messages: iocraft::hooks::State<Vec<(String, String)>>,
    is_processing: iocraft::hooks::State<bool>,
) {
    // Create a channel for UI updates
    let (ui_sender, mut ui_receiver) = mpsc::unbounded_channel();

    // Forward UI messages to the component state
    let mut messages_for_receiver = messages.clone();
    let mut is_processing_for_receiver = is_processing.clone();
    tokio::spawn(async move {
        while let Some(app_message) = ui_receiver.recv().await {
            match app_message {
                AppMessage::AgentExecutionCompleted => {
                    is_processing_for_receiver.set(false);
                }
                _ => {
                    // Convert and add message if applicable
                    if let Some((role, content)) = app_message_to_ui_message(app_message) {
                        let mut current_messages = messages_for_receiver.read().clone();
                        current_messages.push((role, content));
                        messages_for_receiver.set(current_messages);
                    }
                }
            }
        }
    });

    // Execute agent task
    tokio::spawn(async move {
        match execute_agent_task(input, config, project_path, ui_sender.clone()).await {
            Ok(_) => {
                let _ = ui_sender.send(AppMessage::AgentExecutionCompleted);
            }
            Err(e) => {
                let _ = ui_sender.send(AppMessage::SystemMessage(format!("❌ Error: {}", e)));
                let _ = ui_sender.send(AppMessage::AgentExecutionCompleted);
            }
        }
    });
}

/// Interactive mode using iocraft
pub async fn run_rich_interactive(config: Config, project_path: PathBuf) -> Result<()> {
    println!("🎯 Starting Trae Agent Interactive Mode");

    // Store config and project path in a global context for the UI (accessible across threads)
    let _ = INTERACTIVE_CONTEXT.set(InteractiveContext { config, project_path });

    // Run the iocraft-based UI
    tokio::task::spawn_blocking(|| {
        smol::block_on(async {
            element!(TraeApp).render_loop().await
        })
    }).await??;

    Ok(())
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
    let (config, project_path) = get_interactive_context();

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
                        let current = input_value.read().clone();
                        if !current.is_empty() {
                            input_value.set(current[..current.len() - 1].to_string());
                        }
                    }
                    KeyCode::Enter => {
                        let input = input_value.read().clone();
                        if input.trim().is_empty() {
                            return;
                        }

                        // Clear input immediately to prevent visual glitches
                        input_value.set(String::new());

                        // Add user message and update UI state in a single batch
                        let mut current_messages = messages.read().clone();
                        current_messages.push(("user".to_string(), input.clone()));
                        messages.set(current_messages);

                        // Set processing state
                        is_processing.set(true);

                        // Execute agent task asynchronously
                        spawn_ui_agent_task(input, config.clone(), project_path.clone(), messages.clone(), is_processing.clone());
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
            width: 100pct,
            padding: 1,
            position: Position::Relative, // Ensure stable positioning
        ) {
            // Scrollable content area - takes up all available space except bottom fixed area
            View(
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::Hidden, // Prevent content from overflowing
                max_height: 100pct,         // Constrain height to prevent expansion
            ) {
                // Header with TRAE logo - always visible
                View(
                    margin_bottom: 1,
                    flex_direction: FlexDirection::Column,
                    flex_shrink: 0.0, // Prevent logo from shrinking
                ) {
                    View(margin_bottom: 1) {
                        TraeLogo
                    }
                    // Tips (only show when no messages)
                    #(if messages.read().is_empty() {
                        Some(element! {
                            View(
                                flex_direction: FlexDirection::Column,
                                margin_bottom: 1,
                            ) {
                                Text(
                                    content: "Tips for getting started:",
                                    color: Color::White,
                                )
                                Text(
                                    content: "1. Ask questions, edit files, or run commands.",
                                    color: Color::White,
                                )
                                Text(
                                    content: "2. Be specific for the best results.",
                                    color: Color::White,
                                )
                                Text(
                                    content: "3. /help for more information.",
                                    color: Color::White,
                                )
                            }
                        })
                    } else {
                        None
                    })
                }

                // Chat messages area - 支持文本换行，防止UI错乱
                View(
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::Scroll, // Enable scrolling for long content
                    min_height: 0, // Prevent flex item from growing beyond container
                ) {
                #(messages.read().iter().map(|(role, content)| {
                    // 动态获取终端宽度并换行，防止自动换行导致UI错乱
                    let terminal_width = get_terminal_width();
                    let wrapped_lines = wrap_text(content, terminal_width);

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

                // Processing indicator
                #(is_processing.get().then(|| element! {
                    View(margin_bottom: 1) {
                        Text(
                            content: "ℹ Processing...",
                            color: Color::Rgb { r: 100, g: 149, b: 237 },
                        )
                    }
                }))
                }
            }

            // Fixed bottom area for input and status - this should never move
            View(
                flex_shrink: 0.0, // Prevent shrinking
                flex_grow: 0.0,   // Prevent growing
                flex_direction: FlexDirection::Column,
                height: 5,        // Fixed height for input area
                position: Position::Relative, // Ensure stable positioning
            ) {
                // Input area - 简约边框风格，单行高度
                View(
                    border_style: BorderStyle::Round,
                    border_color: Color::Rgb { r: 100, g: 149, b: 237 }, // 蓝色边框
                    padding_left: 1,
                    padding_right: 1,
                    padding_top: 0,
                    padding_bottom: 0,
                    margin_bottom: 1,
                    height: 3, // Fixed height to prevent expansion
                    flex_shrink: 0.0, // Prevent shrinking
                    flex_grow: 0.0,   // Prevent growing
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
}

/// Execute agent task asynchronously and send updates to UI
async fn execute_agent_task(
    task: String,
    config: Config,
    project_path: PathBuf,
    ui_sender: mpsc::UnboundedSender<AppMessage>,
) -> Result<()> {
    use crate::output::interactive_handler::{InteractiveOutputHandler, InteractiveOutputConfig};

    // Get agent configuration
    let agent_config = config.agents.get("trae_agent").cloned().unwrap_or_default();

    // Create channel for InteractiveMessage and forward to AppMessage
    let (interactive_sender, mut interactive_receiver) = mpsc::unbounded_channel();

    // Forward InteractiveMessage to AppMessage
    tokio::spawn(async move {
        while let Some(interactive_msg) = interactive_receiver.recv().await {
            let _ = ui_sender.send(AppMessage::InteractiveUpdate(interactive_msg));
        }
    });

    // Create InteractiveOutputHandler with UI integration
    let interactive_config = InteractiveOutputConfig {
        realtime_updates: true,
        show_tool_details: true,
    };
    let interactive_output = Box::new(InteractiveOutputHandler::new(interactive_config, interactive_sender));

    // Create and execute agent task
    let mut agent = TraeAgent::new_with_output(agent_config, config, interactive_output).await?;
    agent.execute_task_with_context(&task, &project_path).await?;

    Ok(())
}


