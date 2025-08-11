//! Interactive application using iocraft

use crate::interactive::animation::UiAnimationConfig;
use crate::interactive::components::input_section::{InputSection, InputSectionContext};
use crate::interactive::components::logo::output_logo_to_terminal;
use crate::interactive::components::status_line::{DynamicStatusLine, StatusLineContext};
use crate::interactive::message_handler::{app_message_to_ui_message, AppMessage};
use crate::interactive::terminal_output::{output_content_block, overwrite_previous_lines};
use anyhow::Result;
use iocraft::prelude::*;
use std::path::PathBuf;
use tokio::sync::broadcast;
use trae_agent_core::Config;

/// Context for interactive mode - immutable application configuration
#[derive(Debug, Clone)]
struct AppContext {
    config: Config,
    project_path: PathBuf,
    ui_sender: broadcast::Sender<AppMessage>,
    ui_anim: UiAnimationConfig,
}

impl AppContext {
    fn new(
        config: Config,
        project_path: PathBuf,
        ui_sender: broadcast::Sender<AppMessage>,
    ) -> Self {
        let ui_anim = UiAnimationConfig::from_env();

        Self {
            config,
            project_path,
            ui_sender,
            ui_anim,
        }
    }
}

/// Interactive mode using iocraft
pub async fn run_rich_interactive(config: Config, project_path: PathBuf) -> Result<()> {
    // Create UI broadcast channel and app context
    let (ui_sender, _ui_rx) = broadcast::channel::<AppMessage>(256);
    let app_context = AppContext::new(config, project_path, ui_sender);

    // Run the iocraft-based UI with context provider in fullscreen mode
    tokio::task::spawn_blocking(move || {
        smol::block_on(async {
            (element! {
                ContextProvider(value: Context::owned(app_context)) {
                    TraeApp
                }
            })
            .render_loop()
            .await
        })
    })
    .await??;

    Ok(())
}

/// Main TRAE Interactive Application Component
#[component]
fn TraeApp(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // Get stdout handle for header and messages output
    let (stdout, _stderr) = hooks.use_output();

    // Local state for header and messages
    let show_tips = hooks.use_state(|| true);
    let header_rendered = hooks.use_state(|| false);
    let messages = hooks.use_state(|| Vec::<(String, String, Option<String>)>::new());
    // Track line counts for each message to enable proper overwriting
    let message_line_counts = hooks.use_state(|| std::collections::HashMap::<String, usize>::new());

    let (width, _height) = hooks.use_terminal_size();
    // Get current terminal width and reserve space for padding/borders
    let raw_width = if width as usize > 0 {
        width as usize
    } else {
        crossterm::terminal::size()
            .map(|(w, _)| w as usize)
            .unwrap_or(80)
    };
    let terminal_width = raw_width.saturating_sub(6);
    let terminal_width = std::cmp::max(terminal_width, 60);

    // Get app context
    let app_context = hooks.use_context::<AppContext>();
    let ui_sender = app_context.ui_sender.clone();

    // Subscribe to UI events for header tips management
    let ui_sender_tips = ui_sender.clone();
    let mut show_tips_clone = show_tips.clone();
    hooks.use_future(async move {
        let mut rx = ui_sender_tips.subscribe();
        while let Ok(msg) = rx.recv().await {
            if app_message_to_ui_message(msg).is_some() {
                if *show_tips_clone.read() {
                    show_tips_clone.set(false);
                }
            }
        }
    });

    // Output header to stdout when component mounts
    let stdout_clone = stdout.clone();
    let show_tips_for_output = show_tips.clone();
    let mut header_rendered_clone = header_rendered.clone();
    hooks.use_future(async move {
        if !*header_rendered_clone.read() {
            // Use the logo output function from the logo module
            output_logo_to_terminal(&stdout_clone);

            // Output tips if they should be shown
            if *show_tips_for_output.read() {
                stdout_clone.println("Tips for getting started:");
                stdout_clone.println("1. Ask questions, edit files, or run commands.");
                stdout_clone.println("2. Be specific for the best results.");
                stdout_clone.println("3. /help for more information.");
                stdout_clone.println(""); // Empty line for spacing
            }

            header_rendered_clone.set(true);
        }
    });

    // Subscribe to UI events for messages output
    let ui_sender_messages = ui_sender.clone();
    let mut messages_clone = messages.clone();
    let mut message_line_counts_clone = message_line_counts.clone();
    let stdout_messages = stdout.clone();
    hooks.use_future(async move {
        let mut rx = ui_sender_messages.subscribe();
        while let Ok(app_message) = rx.recv().await {
            if let Some((role, content, message_id, is_bash_output)) =
                app_message_to_ui_message(app_message)
            {
                use crate::interactive::message_handler::identify_content_block;

                let mut current = messages_clone.read().clone();
                let mut line_counts = message_line_counts_clone.read().clone();
                let is_new_message = if let Some(msg_id) = &message_id {
                    if let Some(pos) = current
                        .iter()
                        .position(|(_, _, id)| id.as_ref() == Some(msg_id))
                    {
                        current[pos] = (role.clone(), content.clone(), Some(msg_id.clone()));
                        false // Updated existing message
                    } else {
                        current.push((role.clone(), content.clone(), Some(msg_id.clone())));
                        true // New message
                    }
                } else {
                    current.push((role.clone(), content.clone(), None));
                    true // New message
                };

                // Output messages to stdout using block-based formatting
                // For new messages, output normally
                // For updated messages (like tool status changes), we need to handle the replacement
                if is_new_message {
                    let block_type = identify_content_block(&content, &role);
                    let total_lines = output_content_block(
                        &stdout_messages,
                        &content,
                        block_type,
                        terminal_width,
                        is_bash_output,
                    );

                    // Store line count for this message
                    if let Some(msg_id) = &message_id {
                        line_counts.insert(msg_id.clone(), total_lines);
                    }
                } else {
                    // This is an updated message (e.g., tool status change from executing to completed)
                    // We need to replace the previous line with the new content

                    // Get the previous line count for this message
                    let previous_lines = if let Some(msg_id) = &message_id {
                        line_counts.get(msg_id).copied().unwrap_or(2) // Default to 2 lines if not found
                    } else {
                        2 // Default fallback
                    };

                    // Use the helper function to overwrite previous lines
                    let new_total_lines = overwrite_previous_lines(
                        &stdout_messages,
                        &content,
                        &role,
                        terminal_width,
                        previous_lines,
                    );

                    // Update line count for this message
                    if let Some(msg_id) = &message_id {
                        line_counts.insert(msg_id.clone(), new_total_lines);
                    }
                }

                messages_clone.set(current);
                message_line_counts_clone.set(line_counts);
            }
        }
    });

    // Create contexts for child components
    let status_context = StatusLineContext {
        ui_sender: app_context.ui_sender.clone(),
        ui_anim: app_context.ui_anim.clone(),
    };

    let input_context = InputSectionContext {
        config: app_context.config.clone(),
        project_path: app_context.project_path.clone(),
        ui_sender: app_context.ui_sender.clone(),
    };

    element! {
        View(
            key: "main-container",
            flex_direction: FlexDirection::Column,
            height: 100pct,
            width: 100pct,
            padding: 1,
            position: Position::Relative,
            justify_content: JustifyContent::End, // Push content to bottom
        ) {
            // Dynamic status line (isolated component to prevent parent re-rendering)
            DynamicStatusLine(key: "dynamic-status-line", context: status_context.clone())

            // Fixed bottom area for input and status - this should never move
            InputSection(key: "input-section-component", context: input_context.clone())
        }
    }
}
