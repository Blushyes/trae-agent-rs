//! Input section component
//!
//! This module provides the input section component that handles
//! user input and displays the status bar.

use crate::interactive::message_handler::AppMessage;
use iocraft::prelude::*;
use std::path::PathBuf;
use tokio::sync::broadcast;
use trae_agent_core::Config;

#[derive(Clone, Props)]
pub struct InputSectionProps {
    pub context: InputSectionContext,
}

impl Default for InputSectionProps {
    fn default() -> Self {
        Self {
            context: InputSectionContext {
                config: Config::default(),
                project_path: PathBuf::new(),
                ui_sender: tokio::sync::broadcast::channel(1).0,
            },
        }
    }
}

/// Context for the input section component
#[derive(Debug, Clone)]
pub struct InputSectionContext {
    pub config: Config,
    pub project_path: PathBuf,
    pub ui_sender: broadcast::Sender<AppMessage>,
}

/// Spawn agent task execution and broadcast UI events
pub fn spawn_ui_agent_task(
    input: String,
    config: Config,
    project_path: PathBuf,
    ui_sender: broadcast::Sender<AppMessage>,
) {
    use crate::interactive::message_handler::get_random_status_word;
    use crate::interactive::task_executor::execute_agent_task;

    // Start with a random status word
    let _ = ui_sender.send(AppMessage::AgentTaskStarted {
        operation: get_random_status_word(),
    });

    // Create a cancellation token for the timer
    let (cancel_sender, mut cancel_receiver) = tokio::sync::oneshot::channel::<()>();

    // Change status word once after 1 second (unless cancelled)
    let ui_sender_timer = ui_sender.clone();
    tokio::spawn(async move {
        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                let _ = ui_sender_timer.send(AppMessage::AgentTaskStarted {
                    operation: get_random_status_word(),
                });
            }
            _ = &mut cancel_receiver => {
                // Timer cancelled, do nothing
            }
        }
    });

    // Execute agent task
    tokio::spawn(async move {
        match execute_agent_task(input, config, project_path, ui_sender.clone()).await {
            Ok(_) => {
                let _ = cancel_sender.send(()); // Cancel the timer
                let _ = ui_sender.send(AppMessage::AgentExecutionCompleted);
            }
            Err(e) => {
                let _ = cancel_sender.send(()); // Cancel the timer
                                                // Check if it's an interruption error
                if e.to_string().contains("Task interrupted by user") {
                    // Don't show error message for user interruptions
                } else {
                    let _ = ui_sender.send(AppMessage::SystemMessage(format!("❌ Error: {}", e)));
                }
                let _ = ui_sender.send(AppMessage::AgentExecutionCompleted);
            }
        }
    });
}

/// Input Section Component - Fixed bottom area for input and status
#[component]
pub fn InputSection(mut hooks: Hooks, props: &InputSectionProps) -> impl Into<AnyElement<'static>> {
    // Subscribe to keyboard and dispatch events
    let context = &props.context;

    // Local input state
    let input_value = hooks.use_state(|| String::new());
    let is_task_running = hooks.use_state(|| false);
    let current_user_input = hooks.use_state(|| String::new());

    // Subscribe to UI events to track task status
    let ui_sender_status = context.ui_sender.clone();
    let mut is_task_running_clone = is_task_running.clone();
    let mut current_user_input_clone = current_user_input.clone();
    hooks.use_future(async move {
        let mut rx = ui_sender_status.subscribe();
        while let Ok(event) = rx.recv().await {
            match event {
                AppMessage::AgentTaskStarted { .. } => {
                    is_task_running_clone.set(true);
                }
                AppMessage::AgentExecutionCompleted
                | AppMessage::AgentExecutionInterrupted { .. } => {
                    is_task_running_clone.set(false);
                    current_user_input_clone.set(String::new());
                }
                AppMessage::UserMessage(input) => {
                    current_user_input_clone.set(input);
                }
                _ => {}
            }
        }
    });

    let config = context.config.clone();
    let project_path = context.project_path.clone();
    let ui_sender = context.ui_sender.clone();

    hooks.use_terminal_events({
        let mut input_value = input_value;
        let config = config.clone();
        let project_path = project_path.clone();
        let ui_sender = ui_sender.clone();
        let is_task_running = is_task_running.clone();
        let current_user_input = current_user_input.clone();
        move |event| {
            match event {
                TerminalEvent::Key(KeyEvent { code, kind, .. })
                    if kind != KeyEventKind::Release =>
                {
                    match code {
                        KeyCode::Char(c) => {
                            let mut current_input = input_value.read().clone();
                            current_input.push(c);
                            input_value.set(current_input);
                        }
                        KeyCode::Backspace => {
                            let mut current = input_value.read().clone();
                            if current.pop().is_some() {
                                input_value.set(current);
                            }
                        }
                        KeyCode::Esc => {
                            // Handle ESC key - interrupt current task if running
                            if *is_task_running.read() {
                                let user_input = current_user_input.read().clone();
                                let _ = ui_sender
                                    .send(AppMessage::AgentExecutionInterrupted { user_input });
                            }
                        }
                        KeyCode::Enter => {
                            let input = input_value.read().clone();
                            if input.trim().is_empty() {
                                return;
                            }

                            // Clear input immediately
                            input_value.set(String::new());

                            // Broadcast user message and start task
                            let _ = ui_sender.send(AppMessage::UserMessage(input.clone()));
                            spawn_ui_agent_task(
                                input,
                                config.clone(),
                                project_path.clone(),
                                ui_sender.clone(),
                            );
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    });

    element! {
        View(
            key: "input-section",
            flex_shrink: 0.0,
            flex_grow: 0.0,
            flex_direction: FlexDirection::Column,
            height: 5,
            position: Position::Relative,
        ) {
            // Input area - 简约边框风格，单行高度
            View(
                key: "input-container",
                border_style: BorderStyle::Round,
                border_color: Color::Rgb { r: 100, g: 149, b: 237 },
                padding_left: 1,
                padding_right: 1,
                padding_top: 0,
                padding_bottom: 0,
                margin_bottom: 1,
                height: 3,
                flex_shrink: 0.0,
                flex_grow: 0.0,
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
            View(padding: 1) {
                Text(
                    content: "~/projects/trae-agent-rs (main*)                       no sandbox (see /docs)                        trae-2.5-pro (100% context left)",
                    color: Color::DarkGrey,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_section_props_default() {
        let props = InputSectionProps::default();
        // Just ensure it compiles and creates successfully
        let _ = props;
    }
}
