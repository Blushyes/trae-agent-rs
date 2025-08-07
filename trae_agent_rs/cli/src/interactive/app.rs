//! Interactive application using iocraft

use anyhow::Result;
use iocraft::prelude::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;


/// Message types for the interactive app
#[derive(Debug, Clone)]
pub enum AppMessage {
    UserInput(String),
    AgentResponse(String),
    SystemMessage(String),
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
#[derive(Debug)]
pub struct InteractiveApp {
    messages: Arc<Mutex<VecDeque<ChatMessage>>>,
    input_buffer: String,
    is_processing: bool,
    sender: mpsc::UnboundedSender<AppMessage>,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<AppMessage>>>,
}

impl InteractiveApp {
    /// Create a new interactive app
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        Self {
            messages: Arc::new(Mutex::new(VecDeque::new())),
            input_buffer: String::new(),
            is_processing: false,
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
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

                    // Simulate agent processing (placeholder)
                    let response = format!("🤖 I would help you with: {}", input);
                    self.add_message(MessageRole::Agent, response);
                    self.is_processing = false;
                }
                AppMessage::AgentResponse(response) => {
                    self.add_message(MessageRole::Agent, response);
                    self.is_processing = false;
                }
                AppMessage::SystemMessage(msg) => {
                    self.add_message(MessageRole::System, msg);
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
pub async fn run_rich_interactive() -> Result<()> {
    println!("🎯 Starting Trae Agent Interactive Mode");

    // Run the iocraft-based UI
    tokio::task::spawn_blocking(|| {
        smol::block_on(async {
            element!(TraeApp).render_loop().await
        })
    }).await??;

    Ok(())
}

/// Main entry point for interactive mode
pub async fn run_interactive() -> Result<()> {
    run_rich_interactive().await
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

                            // Simulate agent response (placeholder)
                            let response = format!("I would help you with: {}", input);
                            let mut current_messages = messages.read().clone();
                            current_messages.push(("agent".to_string(), response));
                            messages.set(current_messages);
                            is_processing.set(false);
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

            // Chat messages area - 简约版本，无边框，每条消息单独一行
            View(
                flex_grow: 1.0,
                margin_bottom: 1,
                flex_direction: FlexDirection::Column,
            ) {
                #(messages.read().iter().map(|(role, content)| {
                    if role == "user" {
                        element! {
                            View(
                                width: 100pct,
                                margin_bottom: 1,
                            ) {
                                Text(
                                    content: format!("> {}", content),
                                    color: Color::White,
                                )
                            }
                        }
                    } else {
                        element! {
                            View(
                                width: 100pct,
                                margin_bottom: 1,
                            ) {
                                Text(
                                    content: content,
                                    color: Color::White,
                                )
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
