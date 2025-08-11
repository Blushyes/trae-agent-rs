//! Input section component
//!
//! This module provides the input section component that handles
//! user input and displays the status bar.

use crate::interactive::file_search::{
    extract_existing_file_references, extract_search_query, should_show_file_search,
};
use crate::interactive::file_search::{
    DefaultFileSearchProvider, FileSearchProvider, FileSearchResult,
};
use crate::interactive::message_handler::AppMessage;
use iocraft::prelude::*;
use std::cmp::min;
use std::path::PathBuf;
use tokio::sync::broadcast;
use trae_agent_core::Config;
use unicode_width::UnicodeWidthStr;

/// Calculate cursor position (line, column) from text and byte position
/// Returns (line_number, column_number) where both are 1-based
fn calculate_cursor_position(text: &str, byte_pos: usize) -> (usize, usize) {
    if text.is_empty() {
        return (1, 1);
    }

    let safe_pos = byte_pos.min(text.len());
    let text_before_cursor = &text[..safe_pos];

    // Count lines (number of newlines + 1)
    let line_number = text_before_cursor.matches('\n').count() + 1;

    // Find the start of the current line
    let current_line_start = text_before_cursor.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let current_line_text = &text_before_cursor[current_line_start..];

    // Count characters in current line (not bytes)
    let column_number = current_line_text.chars().count() + 1;

    (line_number, column_number)
}

/// Calculate cursor display position for rendering
/// Returns (display_line, display_column) where both are 0-based for UI positioning
fn calculate_cursor_display_position(
    text: &str,
    byte_pos: usize,
    _max_width: usize,
) -> (usize, usize) {
    if text.is_empty() {
        return (0, 0);
    }

    let safe_pos = byte_pos.min(text.len());
    let text_before_cursor = &text[..safe_pos];

    // Count display lines (number of newlines)
    let display_line = text_before_cursor.matches('\n').count();

    // Find the start of the current line
    let current_line_start = text_before_cursor.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let current_line_text = &text_before_cursor[current_line_start..];

    // Count display characters in current line (using Unicode width)
    let display_column = current_line_text.width();

    (display_line, display_column)
}

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

/// Enhanced text input component that wraps iocraft's TextInput with submit handling
#[derive(Props)]
pub struct EnhancedTextInputProps {
    pub value: String,
    pub has_focus: bool,
    pub on_change: Handler<'static, String>,
    pub on_submit: Handler<'static, String>,
    pub on_cursor_position_change: Handler<'static, (usize, usize)>, // (line, column)
    pub width: u16,
    pub placeholder: String,
    pub color: Option<Color>,
    pub cursor_color: Option<Color>,
    pub project_path: PathBuf,
}

impl Default for EnhancedTextInputProps {
    fn default() -> Self {
        Self {
            value: String::new(),
            has_focus: false,
            on_change: Handler::default(),
            on_submit: Handler::default(),
            on_cursor_position_change: Handler::default(),
            width: 80,
            placeholder: String::new(),
            color: None,
            cursor_color: None,
            project_path: PathBuf::new(),
        }
    }
}

/// Simple multiline text input component without internal scrolling
#[component]
pub fn EnhancedTextInput(
    mut hooks: Hooks,
    props: &mut EnhancedTextInputProps,
) -> impl Into<AnyElement<'static>> {
    let has_focus = props.has_focus;
    let width = props.width;
    let project_path = props.project_path.clone();

    // Local state for cursor position
    let cursor_pos = hooks.use_state(|| props.value.len());

    // State for file search popup
    let show_file_list = hooks.use_state(|| false);
    let search_results = hooks.use_state(|| Vec::<FileSearchResult>::new());
    let selected_file_index = hooks.use_state(|| 0usize);
    let current_query = hooks.use_state(|| String::new());

    // Cache for existing file references to avoid repeated parsing
    let cached_existing_refs = hooks.use_state(|| Vec::<String>::new());
    let last_input_for_refs = hooks.use_state(|| String::new());

    // Initialize search provider
    let search_provider =
        hooks.use_state(|| DefaultFileSearchProvider::new(project_path.clone()).ok());

    // Handle keyboard input
    hooks.use_terminal_events({
        let mut on_change = props.on_change.take();
        let mut on_submit = props.on_submit.take();
        let mut on_cursor_position_change = props.on_cursor_position_change.take();
        let mut value = props.value.clone();
        let mut cursor_pos = cursor_pos.clone();
        let mut show_file_list = show_file_list.clone();
        let mut search_results = search_results.clone();
        let mut selected_file_index = selected_file_index.clone();
        let mut current_query = current_query.clone();
        let search_provider = search_provider.clone();
        let _project_path = project_path.clone();
        let mut cached_existing_refs = cached_existing_refs.clone();
        let mut last_input_for_refs = last_input_for_refs.clone();

        move |event| {
            if !has_focus {
                return;
            }

            match event {
                TerminalEvent::Key(KeyEvent {
                    code,
                    modifiers,
                    kind,
                    ..
                }) if kind != KeyEventKind::Release => {
                    let mut pos = cursor_pos.get();
                    let mut changed = false;

                    // Handle file list navigation when it's shown
                    if *show_file_list.read() {
                        match code {
                            KeyCode::Up => {
                                let current = selected_file_index.get();
                                if current > 0 {
                                    selected_file_index.set(current - 1);
                                }
                                return;
                            }
                            KeyCode::Down => {
                                let current = selected_file_index.get();
                                let max_index = search_results.read().len().saturating_sub(1);
                                if current < max_index {
                                    selected_file_index.set(current + 1);
                                }
                                return;
                            }
                            KeyCode::Char('p') if modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+P: Move up (previous)
                                let current = selected_file_index.get();
                                if current > 0 {
                                    selected_file_index.set(current - 1);
                                }
                                return;
                            }
                            KeyCode::Char('n') if modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+N: Move down (next)
                                let current = selected_file_index.get();
                                let max_index = search_results.read().len().saturating_sub(1);
                                if current < max_index {
                                    selected_file_index.set(current + 1);
                                }
                                return;
                            }
                            KeyCode::Enter | KeyCode::Tab => {
                                // Insert selected file
                                if let Some(selected_result) =
                                    search_results.read().get(selected_file_index.get())
                                {
                                    // Find the @ position and replace the entire @search_term with @absolute_path + space
                                    if let Some(query) = extract_search_query(&value, pos) {
                                        if let Some(at_pos) = value.rfind('@') {
                                            // Find the end of the search term
                                            let search_end = at_pos + 1 + query.len();

                                            let before_at = &value[..at_pos];
                                            let after_search = &value[search_end..];

                                            // Create replacement: @absolute_path + space
                                            let replacement =
                                                format!("@{} ", selected_result.insertion_path);

                                            value = format!(
                                                "{}{}{}",
                                                before_at, replacement, after_search
                                            );
                                            pos = at_pos + replacement.len();
                                            cursor_pos.set(pos);
                                            on_change(value.clone());

                                            // Update cursor position
                                            let (line, col) =
                                                calculate_cursor_position(&value, pos);
                                            on_cursor_position_change((line, col));
                                        }
                                    }
                                }
                                show_file_list.set(false);
                                return;
                            }
                            KeyCode::Esc => {
                                show_file_list.set(false);
                                return;
                            }
                            _ => {}
                        }
                    }

                    match code {
                        KeyCode::Char(c) => {
                            // Ensure we're at a character boundary before inserting
                            let safe_pos = pos.min(value.len());
                            let char_pos = value[..safe_pos].chars().count();

                            let mut chars: Vec<char> = value.chars().collect();
                            chars.insert(char_pos, c);
                            value = chars.into_iter().collect();

                            // Update position to after the inserted character
                            pos = value
                                .char_indices()
                                .nth(char_pos + 1)
                                .map(|(i, _)| i)
                                .unwrap_or(value.len());
                            changed = true;

                            // Update cursor position
                            let (line, col) = calculate_cursor_position(&value, pos);
                            on_cursor_position_change((line, col));

                            // Check if we should show/update/hide file list after character input
                            let should_show = should_show_file_search(&value, pos);

                            if should_show {
                                if let Some(query) = extract_search_query(&value, pos) {
                                    // Show list and update search if needed
                                    if !*show_file_list.read() || query != *current_query.read() {
                                        if let Some(search_provider) =
                                            search_provider.read().as_ref()
                                        {
                                            // Get cached existing file references to exclude them
                                            let existing_refs = if value
                                                != *last_input_for_refs.read()
                                            {
                                                let refs =
                                                    extract_existing_file_references(&value, pos);
                                                cached_existing_refs.set(refs.clone());
                                                last_input_for_refs.set(value.clone());
                                                refs
                                            } else {
                                                cached_existing_refs.read().clone()
                                            };
                                            let exclude_paths: Vec<&str> =
                                                existing_refs.iter().map(|s| s.as_str()).collect();

                                            let results = if query.is_empty() {
                                                search_provider
                                                    .get_all_files_with_exclusions(&exclude_paths)
                                            } else {
                                                search_provider
                                                    .search_with_exclusions(&query, &exclude_paths)
                                            };
                                            search_results.set(results);
                                            selected_file_index.set(0);
                                            current_query.set(query);
                                            show_file_list.set(true);
                                        }
                                    }
                                }
                            } else {
                                // Should not show list, hide it
                                show_file_list.set(false);
                            }
                        }
                        KeyCode::Backspace => {
                            if pos > 0 {
                                // Find the start of the previous character
                                let safe_pos = pos.min(value.len());
                                let char_start = value[..safe_pos]
                                    .char_indices()
                                    .last()
                                    .map(|(i, _)| i)
                                    .unwrap_or(0);

                                // Convert to chars, remove the previous character, and rebuild string
                                let mut chars: Vec<char> = value.chars().collect();
                                let char_pos = value[..safe_pos].chars().count();
                                if char_pos > 0 {
                                    chars.remove(char_pos - 1);
                                    value = chars.into_iter().collect();
                                    pos = char_start;
                                    changed = true;

                                    // Update cursor position
                                    let (line, col) = calculate_cursor_position(&value, pos);
                                    on_cursor_position_change((line, col));

                                    // Check if we should show/update/hide file list after backspace
                                    let should_show = should_show_file_search(&value, pos);

                                    if should_show {
                                        if let Some(query) = extract_search_query(&value, pos) {
                                            // Show list and update search if needed
                                            if !*show_file_list.read()
                                                || query != *current_query.read()
                                            {
                                                if let Some(search_provider) =
                                                    search_provider.read().as_ref()
                                                {
                                                    // Get cached existing file references to exclude them
                                                    let existing_refs = if value
                                                        != *last_input_for_refs.read()
                                                    {
                                                        let refs = extract_existing_file_references(
                                                            &value, pos,
                                                        );
                                                        cached_existing_refs.set(refs.clone());
                                                        last_input_for_refs.set(value.clone());
                                                        refs
                                                    } else {
                                                        cached_existing_refs.read().clone()
                                                    };
                                                    let exclude_paths: Vec<&str> = existing_refs
                                                        .iter()
                                                        .map(|s| s.as_str())
                                                        .collect();

                                                    let results = if query.is_empty() {
                                                        search_provider
                                                            .get_all_files_with_exclusions(
                                                                &exclude_paths,
                                                            )
                                                    } else {
                                                        search_provider.search_with_exclusions(
                                                            &query,
                                                            &exclude_paths,
                                                        )
                                                    };
                                                    search_results.set(results);
                                                    selected_file_index.set(0);
                                                    current_query.set(query);
                                                    show_file_list.set(true);
                                                }
                                            }
                                        }
                                    } else {
                                        // Should not show list, hide it
                                        show_file_list.set(false);
                                    }
                                }
                            }
                        }
                        KeyCode::Delete => {
                            if pos < value.len() {
                                // Find the next character boundary to delete safely
                                if value[pos..].chars().next().is_some() {
                                    // Convert to chars, remove one, and rebuild string
                                    let mut chars: Vec<char> = value.chars().collect();
                                    let safe_pos = pos.min(value.len());
                                    let char_pos = value[..safe_pos].chars().count();
                                    if char_pos < chars.len() {
                                        chars.remove(char_pos);
                                        value = chars.into_iter().collect();
                                        changed = true;

                                        // Update cursor position
                                        let (line, col) = calculate_cursor_position(&value, pos);
                                        on_cursor_position_change((line, col));

                                        // Update search or hide file list
                                        if *show_file_list.read() {
                                            if let Some(query) = extract_search_query(&value, pos) {
                                                if query != *current_query.read() {
                                                    if let Some(search_provider) =
                                                        search_provider.read().as_ref()
                                                    {
                                                        // Get cached existing file references to exclude them
                                                        let existing_refs = if value
                                                            != *last_input_for_refs.read()
                                                        {
                                                            let refs =
                                                                extract_existing_file_references(
                                                                    &value, pos,
                                                                );
                                                            cached_existing_refs.set(refs.clone());
                                                            last_input_for_refs.set(value.clone());
                                                            refs
                                                        } else {
                                                            cached_existing_refs.read().clone()
                                                        };
                                                        let exclude_paths: Vec<&str> =
                                                            existing_refs
                                                                .iter()
                                                                .map(|s| s.as_str())
                                                                .collect();

                                                        let results = if query.is_empty() {
                                                            search_provider
                                                                .get_all_files_with_exclusions(
                                                                    &exclude_paths,
                                                                )
                                                        } else {
                                                            search_provider.search_with_exclusions(
                                                                &query,
                                                                &exclude_paths,
                                                            )
                                                        };
                                                        search_results.set(results);
                                                        selected_file_index.set(0);
                                                        current_query.set(query);
                                                    }
                                                }
                                            } else {
                                                // No valid query found, hide the list
                                                show_file_list.set(false);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Enter => {
                            // Check if current line ends with backslash
                            let current_line_end_with_backslash = {
                                // Find the current line by looking backwards from cursor position
                                let safe_pos = pos.min(value.len());
                                let before_cursor = &value[..safe_pos];
                                let current_line_start =
                                    before_cursor.rfind('\n').map(|i| i + 1).unwrap_or(0);

                                // Find the end of current line by looking forward from cursor
                                let after_cursor = &value[pos..];
                                let current_line_end = after_cursor
                                    .find('\n')
                                    .map(|i| pos + i)
                                    .unwrap_or(value.len());

                                // Get the current line content
                                let current_line = &value[current_line_start..current_line_end];
                                current_line.trim_end().ends_with('\\')
                            };

                            if current_line_end_with_backslash {
                                // Remove the trailing backslash and add newline
                                let safe_pos = pos.min(value.len());
                                let before_cursor = &value[..safe_pos];
                                let current_line_start =
                                    before_cursor.rfind('\n').map(|i| i + 1).unwrap_or(0);

                                let after_cursor = &value[pos..];
                                let current_line_end = after_cursor
                                    .find('\n')
                                    .map(|i| pos + i)
                                    .unwrap_or(value.len());

                                let current_line = &value[current_line_start..current_line_end];
                                let trimmed_line = current_line.trim_end();

                                if let Some(backslash_pos) = trimmed_line.rfind('\\') {
                                    // Create new string parts to avoid borrowing conflicts
                                    let new_line = trimmed_line[..backslash_pos].to_string();
                                    let before_line = value[..current_line_start].to_string();
                                    let after_line = value[current_line_end..].to_string();

                                    value = format!("{}{}\n{}", before_line, new_line, after_line);
                                    pos = current_line_start + new_line.len() + 1; // Position after newline
                                    changed = true;

                                    // Update cursor position
                                    let (line, col) = calculate_cursor_position(&value, pos);
                                    on_cursor_position_change((line, col));
                                }
                            } else if modifiers.contains(KeyModifiers::SHIFT) {
                                // Shift+Enter adds newline - use safe character insertion
                                let safe_pos = pos.min(value.len());
                                let char_pos = value[..safe_pos].chars().count();
                                let mut chars: Vec<char> = value.chars().collect();
                                chars.insert(char_pos, '\n');
                                value = chars.into_iter().collect();

                                // Update position to after the inserted newline
                                pos = value
                                    .char_indices()
                                    .nth(char_pos + 1)
                                    .map(|(i, _)| i)
                                    .unwrap_or(value.len());
                                changed = true;

                                // Update cursor position
                                let (line, col) = calculate_cursor_position(&value, pos);
                                on_cursor_position_change((line, col));
                            } else {
                                // Regular Enter submits
                                on_submit(value.clone());
                                return;
                            }
                        }
                        KeyCode::Left => {
                            if pos > 0 {
                                let safe_pos = pos.min(value.len());
                                let char_start = value[..safe_pos]
                                    .char_indices()
                                    .last()
                                    .map(|(i, _)| i)
                                    .unwrap_or(0);
                                pos = char_start;
                                cursor_pos.set(pos);

                                // Update cursor position
                                let (line, col) = calculate_cursor_position(&value, pos);
                                on_cursor_position_change((line, col));
                            }
                        }
                        KeyCode::Right => {
                            if pos < value.len() {
                                let char_end = value[pos..]
                                    .char_indices()
                                    .nth(1)
                                    .map(|(i, _)| pos + i)
                                    .unwrap_or(value.len());
                                pos = char_end;
                                cursor_pos.set(pos);

                                // Update cursor position
                                let (line, col) = calculate_cursor_position(&value, pos);
                                on_cursor_position_change((line, col));
                            }
                        }
                        _ => {}
                    }

                    if changed {
                        cursor_pos.set(pos);
                        on_change(value.clone());
                    }
                }
                _ => {}
            }
        }
    });

    // Split text into display lines with wrapping
    let effective_width = (width as usize).saturating_sub(4); // Account for borders and padding
    let display_lines = if props.value.is_empty() {
        vec![String::new()]
    } else {
        let mut lines = Vec::new();
        for line in props.value.lines() {
            if line.is_empty() {
                lines.push(String::new());
            } else {
                // Simple wrapping: split long lines
                let line_width = UnicodeWidthStr::width(line);
                if line_width <= effective_width {
                    lines.push(line.to_string());
                } else {
                    // Need to wrap this line
                    let mut current_line = String::new();
                    let mut current_width = 0;

                    for ch in line.chars() {
                        let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                        if current_width + char_width > effective_width && !current_line.is_empty()
                        {
                            lines.push(current_line);
                            current_line = String::new();
                            current_width = 0;
                        }
                        current_line.push(ch);
                        current_width += char_width;
                    }
                    if !current_line.is_empty() {
                        lines.push(current_line);
                    }
                }
            }
        }

        if props.value.ends_with('\n') {
            lines.push(String::new());
        }

        lines
    };

    let total_height = (display_lines.len() + 2) as u16; // +2 for borders

    element! {
        View(
            flex_direction: FlexDirection::Column,
            position: Position::Relative,
        ) {
            // Input box
            View(
                width: width,
                height: total_height,
                border_style: BorderStyle::Round,
                border_color: Color::Rgb { r: 100, g: 149, b: 237 },
                padding_left: 1,
                padding_right: 1,
                position: Position::Relative,
            ) {
                // Content area with cursor
                View(
                    flex_direction: FlexDirection::Column,
                    width: 100pct,
                    height: 100pct,
                    position: Position::Relative,
                ) {
                    #(display_lines.iter().enumerate().map(|(line_idx, line)| {
                        element! {
                            View(
                                key: format!("line-{}", line_idx),
                                height: 1,
                                width: 100pct,
                                position: Position::Relative,
                            ) {
                                #(if line.is_empty() && line_idx == 0 && props.value.is_empty() && !props.placeholder.is_empty() {
                                    Some(element! {
                                        Text(
                                            content: &props.placeholder,
                                            color: Color::DarkGrey,
                                        )
                                    })
                                } else {
                                    Some(element! {
                                        Text(
                                            content: line,
                                            color: props.color.unwrap_or(Color::White),
                                        )
                                    })
                                })
                            }
                        }
                    }))

                    // Render cursor if component has focus
                    #(if props.has_focus {
                        // Calculate cursor position in display coordinates
                        let cursor_pos = cursor_pos.get();
                        let (cursor_line, cursor_col) = calculate_cursor_display_position(&props.value, cursor_pos, width as usize - 2);

                        // Get the character at cursor position for semi-transparent effect
                        let cursor_char = if props.value.is_empty() && !props.placeholder.is_empty() {
                            // Show placeholder character at cursor position with different styling
                            props.placeholder.chars().nth(cursor_col).unwrap_or(' ')
                        } else {
                            // Show space or character at cursor position
                            let chars: Vec<char> = props.value.chars().collect();
                            let char_pos = cursor_pos.min(chars.len());
                            if char_pos < chars.len() {
                                chars[char_pos]
                            } else {
                                ' '
                            }
                        };

                        Some(element! {
                            View(
                                key: "cursor",
                                position: Position::Absolute,
                                top: cursor_line as u16,
                                left: cursor_col as u16,
                                width: 1,
                                height: 1,
                                background_color: props.cursor_color.unwrap_or(Color::Rgb { r: 200, g: 200, b: 200 }), // Light grey background
                            ) {
                                Text(
                                    content: cursor_char.to_string(),
                                    color: Color::Rgb { r: 80, g: 80, b: 80 }, // Dark text for contrast
                                )
                            }
                        })
                    } else {
                        None
                    })
                }
            }

            // File list popup
            #(if *show_file_list.read() {
                let results = search_results.read();
                let selected_index = selected_file_index.get();
                let max_display_files = 10;
                let display_results: Vec<_> = results.iter().take(max_display_files).enumerate().collect();

                Some(element! {
                    View(
                        key: "file-list",
                        width: width,
                        height: min(results.len(), max_display_files) as u16,
                        position: Position::Relative,
                    ) {
                        View(
                            flex_direction: FlexDirection::Column,
                            width: 100pct,
                            height: 100pct,
                            padding_left: 2,
                            padding_right: 2,
                        ) {
                            #(display_results.iter().map(|(idx, result)| {
                                let is_selected = *idx == selected_index;
                                element! {
                                    View(
                                        key: format!("file-{}", idx),
                                        height: 1,
                                        width: 100pct,
                                    ) {
                                        Text(
                                            content: result.display_name.clone(),
                                            color: if is_selected {
                                                Color::Rgb { r: 100, g: 149, b: 237 }
                                            } else {
                                                Color::DarkGrey
                                            },
                                        )
                                    }
                                }
                            }))
                        }
                    }
                })
            } else {
                None
            })
        }
    }
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

    // Get terminal width for fixed input width
    let (terminal_width, _height) = hooks.use_terminal_size();
    let input_width = if terminal_width > 6 {
        terminal_width - 4 // Reserve space for padding and borders
    } else {
        80 // Fallback width
    };

    // Local input state
    let input_value = hooks.use_state(|| String::new());
    let is_task_running = hooks.use_state(|| false);
    let current_user_input = hooks.use_state(|| String::new());
    let cursor_position = hooks.use_state(|| (1usize, 1usize)); // (line, column)

    // Initialize cursor position when input value changes
    let mut cursor_position_init = cursor_position.clone();
    let input_value_for_init = input_value.clone();
    hooks.use_future(async move {
        let current_input = input_value_for_init.read().clone();
        let (line, col) = calculate_cursor_position(&current_input, current_input.len());
        cursor_position_init.set((line, col));
    });

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

    // Handle ESC key for task interruption
    hooks.use_terminal_events({
        let ui_sender = ui_sender.clone();
        let is_task_running = is_task_running.clone();
        let current_user_input = current_user_input.clone();
        move |event| {
            match event {
                TerminalEvent::Key(KeyEvent { code, kind, .. })
                    if kind != KeyEventKind::Release =>
                {
                    match code {
                        KeyCode::Esc => {
                            // Handle ESC key - interrupt current task if running
                            if *is_task_running.read() {
                                let user_input = current_user_input.read().clone();
                                let _ = ui_sender
                                    .send(AppMessage::AgentExecutionInterrupted { user_input });
                            }
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
            position: Position::Relative,
        ) {
            // Enhanced multiline input area with fixed width and auto height
            EnhancedTextInput(
                key: "enhanced-text-input",
                value: input_value.to_string(),
                has_focus: !*is_task_running.read(),
                width: input_width,
                placeholder: "Type your message or @path/to/file (Enter to send, Shift+Enter for new line)".to_string(),
                color: Some(Color::White),
                cursor_color: Some(Color::Rgb { r: 100, g: 149, b: 237 }),
                project_path: project_path.clone(),
                on_change: {
                    let mut input_value = input_value.clone();
                    move |new_value| {
                        input_value.set(new_value);
                    }
                },
                on_cursor_position_change: {
                    let mut cursor_position = cursor_position.clone();
                    move |(line, col)| {
                        cursor_position.set((line, col));
                    }
                },
                on_submit: {
                    let mut input_value = input_value.clone();
                    let mut cursor_position = cursor_position.clone();
                    let ui_sender = ui_sender.clone();
                    let config = config.clone();
                    let project_path = project_path.clone();
                    move |input: String| {
                        if input.trim().is_empty() {
                            return;
                        }

                        // Clear input immediately and reset cursor position
                        input_value.set(String::new());
                        cursor_position.set((1, 1));

                        // Use enhanced task submission with file reference processing
                        crate::interactive::app::submit_task_with_file_processing(
                            input,
                            config.clone(),
                            project_path.clone(),
                            ui_sender.clone(),
                        );
                    }
                },
            )

            // Status bar - 简约风格
            View(padding: 1) {
                Text(
                    content: {
                        let (line, col) = cursor_position.get();
                        format!(
                            "~/projects/trae-agent-rs (main*)  Ln {}, Col {}                no sandbox (see /docs)",
                            line, col
                        )
                    },
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

    #[test]
    fn test_calculate_cursor_position() {
        // Test empty string
        assert_eq!(calculate_cursor_position("", 0), (1, 1));

        // Test single line
        assert_eq!(calculate_cursor_position("hello", 0), (1, 1));
        assert_eq!(calculate_cursor_position("hello", 2), (1, 3));
        assert_eq!(calculate_cursor_position("hello", 5), (1, 6));

        // Test multiple lines
        let multiline = "line1\nline2\nline3";
        assert_eq!(calculate_cursor_position(multiline, 0), (1, 1)); // Start of first line
        assert_eq!(calculate_cursor_position(multiline, 5), (1, 6)); // End of first line
        assert_eq!(calculate_cursor_position(multiline, 6), (2, 1)); // Start of second line
        assert_eq!(calculate_cursor_position(multiline, 11), (2, 6)); // End of second line
        assert_eq!(calculate_cursor_position(multiline, 12), (3, 1)); // Start of third line
        assert_eq!(calculate_cursor_position(multiline, 17), (3, 6)); // End of third line

        // Test Unicode characters
        let unicode_text = "你好\n世界";
        assert_eq!(calculate_cursor_position(unicode_text, 0), (1, 1)); // Start
        assert_eq!(calculate_cursor_position(unicode_text, 3), (1, 2)); // After first char
        assert_eq!(calculate_cursor_position(unicode_text, 6), (1, 3)); // After second char
        assert_eq!(calculate_cursor_position(unicode_text, 7), (2, 1)); // Start of second line
        assert_eq!(calculate_cursor_position(unicode_text, 13), (2, 3)); // End
    }

    #[test]
    fn test_calculate_cursor_display_position() {
        // Test empty string
        assert_eq!(calculate_cursor_display_position("", 0, 80), (0, 0));

        // Test single line
        assert_eq!(calculate_cursor_display_position("hello", 0, 80), (0, 0));
        assert_eq!(calculate_cursor_display_position("hello", 2, 80), (0, 2));
        assert_eq!(calculate_cursor_display_position("hello", 5, 80), (0, 5));

        // Test multiple lines
        let multiline = "line1\nline2\nline3";
        assert_eq!(calculate_cursor_display_position(multiline, 0, 80), (0, 0)); // Start of first line
        assert_eq!(calculate_cursor_display_position(multiline, 5, 80), (0, 5)); // End of first line
        assert_eq!(calculate_cursor_display_position(multiline, 6, 80), (1, 0)); // Start of second line
        assert_eq!(calculate_cursor_display_position(multiline, 11, 80), (1, 5)); // End of second line
        assert_eq!(calculate_cursor_display_position(multiline, 12, 80), (2, 0)); // Start of third line
        assert_eq!(calculate_cursor_display_position(multiline, 17, 80), (2, 5)); // End of third line

        // Test Unicode characters (Chinese characters have width 2)
        let unicode_text = "你好\n世界";
        assert_eq!(
            calculate_cursor_display_position(unicode_text, 0, 80),
            (0, 0)
        ); // Start
        assert_eq!(
            calculate_cursor_display_position(unicode_text, 3, 80),
            (0, 2)
        ); // After first char (width 2)
        assert_eq!(
            calculate_cursor_display_position(unicode_text, 6, 80),
            (0, 4)
        ); // After second char (width 2)
        assert_eq!(
            calculate_cursor_display_position(unicode_text, 7, 80),
            (1, 0)
        ); // Start of second line
        assert_eq!(
            calculate_cursor_display_position(unicode_text, 13, 80),
            (1, 4)
        ); // End (width 2 per char)
    }
}
