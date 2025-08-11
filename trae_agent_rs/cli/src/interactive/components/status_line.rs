//! Dynamic status line component
//!
//! This module provides the dynamic status line component that shows
//! agent execution status, progress, and token usage.

use crate::interactive::animation::{
    apply_easing, SpinnerAnimation, TokenAnimation, UiAnimationConfig,
};
use crate::interactive::message_handler::AppMessage;
use iocraft::prelude::*;
use tokio::sync::broadcast;

#[derive(Clone, Props)]
pub struct DynamicStatusLineProps {
    pub context: StatusLineContext,
}

impl Default for DynamicStatusLineProps {
    fn default() -> Self {
        Self {
            context: StatusLineContext {
                ui_sender: tokio::sync::broadcast::channel(1).0,
                ui_anim: UiAnimationConfig::default(),
            },
        }
    }
}

/// Context for the status line component
#[derive(Debug, Clone)]
pub struct StatusLineContext {
    pub ui_sender: broadcast::Sender<AppMessage>,
    pub ui_anim: UiAnimationConfig,
}

/// Dynamic Status Line Component (Isolated to prevent parent re-rendering)
#[component]
pub fn DynamicStatusLine(
    mut hooks: Hooks,
    props: &DynamicStatusLineProps,
) -> impl Into<AnyElement<'static>> {
    // Local state
    let is_processing = hooks.use_state(|| false);
    let operation = hooks.use_state(|| String::new());
    let start_time = hooks.use_state(|| std::time::Instant::now());
    let current_tokens = hooks.use_state(|| 0u32);
    let target_tokens = hooks.use_state(|| 0u32);
    let token_animation_start = hooks.use_state(|| std::time::Instant::now());

    // Get animation config from props
    let context = &props.context;
    let ui_duration_ms = context.ui_anim.duration_ms;
    let token_animation_duration =
        hooks.use_state(|| std::time::Duration::from_millis(ui_duration_ms));

    // Subscribe to UI events (clone only the sender to avoid non-Send context capture)
    let ui_sender = context.ui_sender.clone();
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
    let anim_cfg = context.ui_anim.clone();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_line_props_default() {
        let props = DynamicStatusLineProps::default();
        // Just ensure it compiles and creates successfully
        let _ = props;
    }
}
