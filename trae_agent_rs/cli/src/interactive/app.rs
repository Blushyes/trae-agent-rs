//! Interactive application using iocraft

use crate::output::interactive_handler::{InteractiveMessage, InteractiveOutputHandler};
use anyhow::Result;
use iocraft::prelude::*;
use std::path::PathBuf;
use tokio::sync::{broadcast, mpsc};
use trae_agent_core::Config;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Easing options for token animation
#[derive(Debug, Clone, Copy)]
enum Easing {
    Linear,
    EaseOutCubic,
    EaseInOutCubic,
}

fn apply_easing(easing: Easing, t: f64) -> f64 {
    match easing {
        Easing::Linear => t,
        Easing::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
        Easing::EaseInOutCubic => {
            if t < 0.5 {
                4.0 * t * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
            }
        }
    }
}

/// Get terminal width with fallback (used as fallback only)
fn get_terminal_width() -> usize {
    match crossterm::terminal::size() {
        Ok((cols, _)) => {
            // Reserve space for padding and borders, and ensure minimum width
            let usable_width = (cols as usize).saturating_sub(12); // 12 chars for padding/borders/safety
            std::cmp::max(usable_width, 30) // Minimum 30 chars
        }
        Err(_) => 68, // Fallback to 68 columns (80 - 12 for safety)
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
                        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
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

/// Context for interactive mode - immutable application configuration
#[derive(Debug, Clone)]
struct UiAnimationConfig {
    easing: Easing,
    frame_interval_ms: u64,
    duration_ms: u64,
}

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
        // Load UI animation config from env (fallback to defaults)
        let easing = std::env::var("TRAE_UI_EASING")
            .ok()
            .and_then(|v| match v.to_lowercase().as_str() {
                "linear" => Some(Easing::Linear),
                "ease_in_out_cubic" | "easeinoutcubic" | "ease-in-out-cubic" => {
                    Some(Easing::EaseInOutCubic)
                }
                "ease_out_cubic" | "easeoutcubic" | "ease-out-cubic" => Some(Easing::EaseOutCubic),
                _ => None,
            })
            .unwrap_or(Easing::EaseOutCubic);
        let frame_interval_ms = std::env::var("TRAE_UI_FRAME_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(10);
        let duration_ms = std::env::var("TRAE_UI_DURATION_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(3000);

        let ui_anim = UiAnimationConfig {
            easing,
            frame_interval_ms,
            duration_ms,
        };

        Self {
            config,
            project_path,
            ui_sender,
            ui_anim,
        }
    }
}

/// Message types for the interactive app
#[derive(Debug, Clone)]
pub enum AppMessage {
    SystemMessage(String),
    UserMessage(String),
    InteractiveUpdate(InteractiveMessage),
    AgentTaskStarted { operation: String },
    AgentExecutionCompleted,
    TokenUpdate { tokens: u32 },
}

/// Generate a unique message ID
fn generate_message_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("msg_{}", timestamp)
}

/// Convert AppMessage to UI message tuple (role, content, message_id)
fn app_message_to_ui_message(app_message: AppMessage) -> Option<(String, String, Option<String>)> {
    match app_message {
        AppMessage::SystemMessage(msg) => {
            Some(("system".to_string(), msg, Some(generate_message_id())))
        }
        AppMessage::UserMessage(msg) => {
            Some(("user".to_string(), msg, Some(generate_message_id())))
        }
        AppMessage::InteractiveUpdate(interactive_msg) => match interactive_msg {
            InteractiveMessage::AgentThinking(thinking) => {
                Some(("agent".to_string(), thinking, Some(generate_message_id())))
            }
            InteractiveMessage::ToolStatus {
                execution_id,
                status,
            } => Some(("system".to_string(), status, Some(execution_id))),
            InteractiveMessage::ToolResult(result) => {
                Some(("agent".to_string(), result, Some(generate_message_id())))
            }
            InteractiveMessage::SystemMessage(msg) => {
                Some(("system".to_string(), msg, Some(generate_message_id())))
            }
            InteractiveMessage::TaskCompleted { success, summary } => {
                let status_icon = if success { "✅" } else { "❌" };
                Some((
                    "system".to_string(),
                    format!("{} Task completed: {}", status_icon, summary),
                    Some(generate_message_id()),
                ))
            }
            InteractiveMessage::ExecutionStats {
                steps,
                duration,
                tokens,
            } => {
                let mut stats = format!("📈 Executed {} steps in {:.2}s", steps, duration);
                if let Some(token_info) = tokens {
                    stats.push_str(&format!("\n{}", token_info));
                }
                Some(("system".to_string(), stats, Some(generate_message_id())))
            }
        },
        AppMessage::AgentTaskStarted { .. } => None,
        AppMessage::AgentExecutionCompleted => None,
        AppMessage::TokenUpdate { .. } => None, // Token updates don't create UI messages, they update state directly
    }
}

/// Spawn agent task execution and broadcast UI events
fn spawn_ui_agent_task(
    input: String,
    config: Config,
    project_path: PathBuf,
    ui_sender: broadcast::Sender<AppMessage>,
) {
    // Notify start
    let _ = ui_sender.send(AppMessage::AgentTaskStarted {
        operation: "Considering".to_string(),
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

// Static logo constant to prevent re-creation
const TRAE_LOGO: &str = r#"
 ███
░░░███
  ░░░███
    ░░░███
     ███░
   ███░
 ███░
░░░
"#;

/// TRAE ASCII Art Logo Component (Static to prevent re-rendering)
#[component]
fn TraeLogo(_hooks: Hooks) -> impl Into<AnyElement<'static>> {
    element! {
        View(key: "logo-content") {
            Text(
                content: TRAE_LOGO,
                color: Color::Rgb { r: 0, g: 255, b: 127 }, // 使用更鲜艳的绿色渐变
                weight: Weight::Bold,
            )
        }
    }
}

#[derive(Clone, Props)]
struct DynamicStatusLineProps {}

impl Default for DynamicStatusLineProps {
    fn default() -> Self {
        Self {}
    }
}

#[derive(Clone, Props)]
struct HeaderSectionProps {}

impl Default for HeaderSectionProps {
    fn default() -> Self {
        Self {}
    }
}

/// Header Section Component - Contains logo and tips
#[component]
fn HeaderSection(mut hooks: Hooks, _props: &HeaderSectionProps) -> impl Into<AnyElement<'static>> {
    // Local state: tips should be shown until the first UI message appears
    let show_tips = hooks.use_state(|| true);

    // Subscribe to UI broadcast and hide tips when any UI message appears
    let ui_sender = hooks.use_context::<AppContext>().ui_sender.clone();
    let mut show_tips_clone = show_tips.clone();
    hooks.use_future(async move {
        let mut rx = ui_sender.subscribe();
        while let Ok(msg) = rx.recv().await {
            if app_message_to_ui_message(msg).is_some() {
                if *show_tips_clone.read() {
                    show_tips_clone.set(false);
                }
            }
        }
    });

    element! {
        View(
            key: "header-section",
            margin_bottom: 1,
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0, // Prevent logo from shrinking
        ) {
            View(key: "logcontanier", margin_bottom: 1) {
                TraeLogo(key: "static-logo")
            }
            // Tips (only show when no messages)
            #(if *show_tips.read() {
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
    }
}

#[derive(Clone, Props)]
struct MessagesAreaProps {}

impl Default for MessagesAreaProps {
    fn default() -> Self {
        Self {}
    }
}

/// Messages Area Component - Chat messages with text wrapping
#[component]
fn MessagesArea(mut hooks: Hooks, _props: &MessagesAreaProps) -> impl Into<AnyElement<'static>> {
    // Local state: messages and terminal width
    let messages = hooks.use_state(|| Vec::<(String, String, Option<String>)>::new());

    let (width, _height) = hooks.use_terminal_size();
    // Get current terminal width and reserve space for padding/borders
    // Subtract more space to prevent line wrapping issues
    let raw_width = if width as usize > 0 {
        width as usize
    } else {
        crossterm::terminal::size()
            .map(|(w, _)| w as usize)
            .unwrap_or(80)
    };

    // Reserve space but be more conservative - only subtract 4-6 chars for basic padding
    let terminal_width = raw_width.saturating_sub(6);
    // Ensure reasonable minimum width
    let terminal_width = std::cmp::max(terminal_width, 60);

    // Subscribe to UI events and update local messages state
    let ui_sender = hooks.use_context::<AppContext>().ui_sender.clone();
    let mut messages_clone = messages.clone();
    hooks.use_future(async move {
        let mut rx = ui_sender.subscribe();
        while let Ok(app_message) = rx.recv().await {
            if let Some((role, content, message_id)) = app_message_to_ui_message(app_message) {
                let mut current = messages_clone.read().clone();
                if let Some(msg_id) = message_id {
                    if let Some(pos) = current
                        .iter()
                        .position(|(_, _, id)| id.as_ref() == Some(&msg_id))
                    {
                        current[pos] = (role, content, Some(msg_id));
                    } else {
                        current.push((role, content, Some(msg_id)));
                    }
                } else {
                    current.push((role, content, None));
                }
                messages_clone.set(current);
            }
        }
    });

    element! {
        View(
            key: "messages-container",
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            overflow: Overflow::Scroll, // Enable scrolling for long content
            min_height: 0, // Prevent flex item from growing beyond container
            position: Position::Relative, // Stable positioning
            flex_shrink: 1.0, // Allow shrinking when needed
        ) {
            #(messages.read().iter().enumerate().map(|(idx, (role, content, message_id))| {
                let wrapped_lines = wrap_text(content, terminal_width);

                // 使用message_id或者索引作为key
                let msg_key = message_id.as_ref().map(|id| id.clone()).unwrap_or_else(|| idx.to_string());

                if role == "user" {
                    element! {
                        View(
                            key: format!("user-msg-{}", msg_key),
                            width: 100pct,
                            margin_bottom: 1,
                            flex_direction: FlexDirection::Column,
                        ) {
                            #(wrapped_lines.iter().enumerate().map(|(i, line)| {
                                element! {
                                    View(key: format!("user-line-{}-{}", msg_key, i), width: 100pct) {
                                        Text(
                                            content: if i == 0 { format!("> {}", line) } else { format!("  {}", line) },
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
                            key: format!("assistant-msg-{}", msg_key),
                            width: 100pct,
                            margin_bottom: 1,
                            flex_direction: FlexDirection::Column,
                        ) {
                            #(wrapped_lines.iter().enumerate().map(|(i, line)| {
                                element! {
                                    View(key: format!("assistant-line-{}-{}", msg_key, i), width: 100pct) {
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
    }
}

#[derive(Clone, Props)]
struct InputSectionProps {}

impl Default for InputSectionProps {
    fn default() -> Self {
        Self {}
    }
}

/// Input Section Component - Fixed bottom area for input and status
#[component]
fn InputSection(mut hooks: Hooks, _props: &InputSectionProps) -> impl Into<AnyElement<'static>> {
    // Local input state
    let input_value = hooks.use_state(|| String::new());

    // Subscribe to keyboard and dispatch events
    let app_context = hooks.use_context::<AppContext>();
    let config = app_context.config.clone();
    let project_path = app_context.project_path.clone();
    let ui_sender = app_context.ui_sender.clone();

    hooks.use_terminal_events({
        let mut input_value = input_value;
        let config = config.clone();
        let project_path = project_path.clone();
        let ui_sender = ui_sender.clone();
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

/// Dynamic Status Line Component (Isolated to prevent parent re-rendering)
#[component]
fn DynamicStatusLine(
    mut hooks: Hooks,
    _props: &DynamicStatusLineProps,
) -> impl Into<AnyElement<'static>> {
    // Local state
    let is_processing = hooks.use_state(|| false);
    let operation = hooks.use_state(|| String::new());
    let start_time = hooks.use_state(|| std::time::Instant::now());
    let current_tokens = hooks.use_state(|| 0u32);
    let target_tokens = hooks.use_state(|| 0u32);
    let token_animation_start = hooks.use_state(|| std::time::Instant::now());
    // 默认时长 3s，可通过 AppContext 配置覆盖
    let ui_duration_ms = hooks.use_context::<AppContext>().ui_anim.duration_ms;
    let token_animation_duration =
        hooks.use_state(|| std::time::Duration::from_millis(ui_duration_ms));

    // Subscribe to UI events (clone only the sender to avoid non-Send context capture)
    let ui_sender = hooks.use_context::<AppContext>().ui_sender.clone();
    let mut is_processing_clone = is_processing.clone();
    let mut operation_clone = operation.clone();
    let mut start_time_clone = start_time.clone();
    let mut current_tokens_clone = current_tokens.clone();
    let mut target_tokens_clone = target_tokens.clone();
    let mut token_animation_start_clone = token_animation_start.clone();
    hooks.use_future(async move {
        let mut rx = ui_sender.subscribe();
        while let Ok(event) = rx.recv().await {
            match event {
                AppMessage::AgentTaskStarted { operation } => {
                    is_processing_clone.set(true);
                    operation_clone.set(operation);
                    start_time_clone.set(std::time::Instant::now());
                    current_tokens_clone.set(0);
                    target_tokens_clone.set(0);
                }
                AppMessage::AgentExecutionCompleted => {
                    is_processing_clone.set(false);
                    operation_clone.set(String::new());
                }
                AppMessage::TokenUpdate { tokens } => {
                    target_tokens_clone.set(tokens);
                    token_animation_start_clone.set(std::time::Instant::now());
                }
                AppMessage::SystemMessage(_)
                | AppMessage::UserMessage(_)
                | AppMessage::InteractiveUpdate(_) => {
                    // Ignored for status line
                }
            }
        }
    });

    // Timer for elapsed and spinner
    let timer_tick = hooks.use_state(|| 0u64);
    let mut timer_tick_clone = timer_tick.clone();
    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            timer_tick_clone.set(timer_tick_clone.get() + 1);
        }
    });

    // Token animation loop using configured frame interval and easing
    let mut current_tokens_anim = current_tokens.clone();
    let token_animation_start_anim = token_animation_start.clone();
    let token_animation_duration_anim = token_animation_duration.clone();
    let target_tokens_anim = target_tokens.clone();
    let anim_cfg = hooks.use_context::<AppContext>().ui_anim.clone();
    hooks.use_future(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(anim_cfg.frame_interval_ms)).await;
            let current = *current_tokens_anim.read();
            let target = *target_tokens_anim.read();
            if current < target {
                let elapsed = token_animation_start_anim.read().elapsed();
                let duration = *token_animation_duration_anim.read();
                if elapsed < duration {
                    let progress = elapsed.as_secs_f64() / duration.as_secs_f64();
                    let eased_progress = apply_easing(anim_cfg.easing, progress);
                    let new_tokens = ((target as f64) * eased_progress) as u32;
                    let calculated = new_tokens.min(target);
                    if calculated != current && calculated > current {
                        current_tokens_anim.set(calculated);
                    }
                } else if current != target {
                    current_tokens_anim.set(target);
                }
            }
        }
    });

    if !*is_processing.read() || operation.read().is_empty() {
        return element! { View {} };
    }

    let elapsed = start_time.read().elapsed().as_secs();
    let spinner_chars = ["✻", "✦", "✧", "✶"];
    let spinner_index = ((elapsed / 1) % 4) as usize;
    let spinner = spinner_chars[spinner_index];
    let status_text = format!(
        "{} {}… ({}s · ↑ {} tokens · esc to interrupt)",
        spinner,
        &*operation.read(),
        elapsed,
        *current_tokens.read(),
    );

    element! {
        View(padding_left: 1, padding_right: 1, margin_bottom: 1) {
            Text(content: status_text, color: Color::Yellow, weight: Weight::Bold)
        }
    }
}

/// Main TRAE Interactive Application Component
#[component]
fn TraeApp(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // Get terminal size for full screen rendering
    let (width, height) = hooks.use_terminal_size();

    element! {
        View(
            key: "main-container",
            flex_direction: FlexDirection::Column,
            width: width,
            height: height,
            padding: 1,
            position: Position::Relative, // Ensure stable positioning
        ) {
            // Header with TRAE logo - always visible
            HeaderSection(key: "header-section-component")

            // Chat messages area - 支持文本换行，防止UI错乱
            MessagesArea(key: "messages-area-component")

            // Dynamic status line (isolated component to prevent parent re-rendering)
            DynamicStatusLine(key: "dynamic-status-line")

            // Fixed bottom area for input and status - this should never move
            InputSection(key: "input-section-component")
        }
    }
}

/// Custom output handler that forwards events and tracks tokens
struct TokenTrackingOutputHandler {
    interactive_handler: InteractiveOutputHandler,
    ui_sender: broadcast::Sender<AppMessage>,
}

impl TokenTrackingOutputHandler {
    fn new(
        interactive_config: crate::output::interactive_handler::InteractiveOutputConfig,
        interactive_sender: mpsc::UnboundedSender<InteractiveMessage>,
        ui_sender: broadcast::Sender<AppMessage>,
    ) -> Self {
        Self {
            interactive_handler: InteractiveOutputHandler::new(
                interactive_config,
                interactive_sender,
            ),
            ui_sender,
        }
    }
}

#[async_trait::async_trait]
impl trae_agent_core::output::AgentOutput for TokenTrackingOutputHandler {
    async fn emit_event(
        &self,
        event: trae_agent_core::output::AgentEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check for token updates in various events
        match &event {
            trae_agent_core::output::AgentEvent::ExecutionCompleted { context, .. } => {
                if context.token_usage.total_tokens > 0 {
                    let _ = self.ui_sender.send(AppMessage::TokenUpdate {
                        tokens: context.token_usage.total_tokens,
                    });
                }
            }
            trae_agent_core::output::AgentEvent::TokenUsageUpdated { token_usage } => {
                // Send immediate token update for smooth animation
                let _ = self.ui_sender.send(AppMessage::TokenUpdate {
                    tokens: token_usage.total_tokens,
                });
            }
            _ => {}
        }

        // Forward to the interactive handler
        self.interactive_handler.emit_event(event).await
    }
}

/// Execute agent task asynchronously and send updates to UI
async fn execute_agent_task(
    task: String,
    config: Config,
    project_path: PathBuf,
    ui_sender: broadcast::Sender<AppMessage>,
) -> Result<()> {
    use crate::output::interactive_handler::InteractiveOutputConfig;

    // Get agent configuration
    let agent_config = config.agents.get("trae_agent").cloned().unwrap_or_default();

    // Create channel for InteractiveMessage and forward to AppMessage
    let (interactive_sender, mut interactive_receiver) = mpsc::unbounded_channel();
    let ui_sender_clone = ui_sender.clone();

    // Forward InteractiveMessage to AppMessage
    tokio::spawn(async move {
        while let Some(interactive_msg) = interactive_receiver.recv().await {
            let _ = ui_sender_clone.send(AppMessage::InteractiveUpdate(interactive_msg));
        }
    });

    // Create TokenTrackingOutputHandler with UI integration
    let interactive_config = InteractiveOutputConfig {
        realtime_updates: true,
        show_tool_details: true,
    };
    let token_tracking_output = Box::new(TokenTrackingOutputHandler::new(
        interactive_config,
        interactive_sender,
        ui_sender,
    ));

    // Create and execute agent task
    let mut agent = trae_agent_core::agent::TraeAgent::new_with_output(
        agent_config,
        config,
        token_tracking_output,
    )
    .await?;
    agent
        .execute_task_with_context(&task, &project_path)
        .await?;

    Ok(())
}
