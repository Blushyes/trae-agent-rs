//! Terminal output abstraction and formatting utilities
//!
//! This module provides terminal output abstractions and formatting functions
//! that work with the AgentOutput system while providing terminal-specific features.

use super::message_handler::ContentBlock;
use super::text_utils::wrap_text;

/// Trait to abstract over different output handles (StdoutHandle, StderrHandle)
pub trait OutputHandle {
    fn println<S: ToString>(&self, msg: S);
}

/// Implementation for iocraft's StdoutHandle
impl OutputHandle for iocraft::hooks::StdoutHandle {
    fn println<S: ToString>(&self, msg: S) {
        self.println(msg);
    }
}

/// Implementation for iocraft's StderrHandle
impl OutputHandle for iocraft::hooks::StderrHandle {
    fn println<S: ToString>(&self, msg: S) {
        self.println(msg);
    }
}

/// Format and output a content block with appropriate spacing
pub fn output_content_block<T: OutputHandle>(
    stdout: &T,
    content: &str,
    block_type: ContentBlock,
    terminal_width: usize,
    is_bash_output: bool,
) -> usize {
    let wrapped_lines = wrap_text(content, terminal_width);

    // Output the content lines
    match block_type {
        ContentBlock::UserInput => {
            for (i, line) in wrapped_lines.iter().enumerate() {
                if i == 0 {
                    stdout.println(format!("> {}", line));
                } else {
                    stdout.println(format!("  {}", line));
                }
            }
        }
        _ => {
            for line in wrapped_lines.iter() {
                if is_bash_output {
                    // Apply gray color using ANSI escape codes
                    let gray_line = format!("\x1b[90m{}\x1b[0m", line);
                    stdout.println(gray_line);
                } else {
                    stdout.println(line);
                }
            }
        }
    }

    // Add empty line after each block for proper spacing, except for ToolStatus
    if !matches!(block_type, ContentBlock::ToolStatus) {
        stdout.println("");
        // Return total lines including the empty line
        wrapped_lines.len() + 1
    } else {
        // For ToolStatus, do not add a trailing empty line so that the following
        // ToolResult can visually attach to it as a single block
        wrapped_lines.len()
    }
}

/// Overwrite previous lines in terminal output using ANSI escape sequences with block-based formatting
///
/// # Parameters
/// - `stdout`: Output handler to write to
/// - `content`: New content to display
/// - `role`: Role of the message sender
/// - `terminal_width`: Width for text wrapping
/// - `previous_line_count`: Number of lines the previous message occupied
///
/// # Returns
/// The number of lines the new content occupies (including empty line)
pub fn overwrite_previous_lines<T: OutputHandle>(
    stdout: &T,
    content: &str,
    role: &str,
    terminal_width: usize,
    previous_line_count: usize,
) -> usize {
    use super::message_handler::{identify_content_block, is_bash_output_content};

    let block_type = identify_content_block(content, role);
    let is_bash_output = is_bash_output_content(content);
    let wrapped_lines = wrap_text(content, terminal_width);

    // Move cursor up to overwrite the previous message and clear from cursor to end of screen
    for (i, line) in wrapped_lines.iter().enumerate() {
        if i == 0 {
            // For the first line, move up and clear from cursor to end of screen, then write new content
            let formatted_line = match block_type {
                ContentBlock::UserInput => format!("> {}", line),
                _ => {
                    if is_bash_output {
                        format!("\x1b[90m{}\x1b[0m", line)
                    } else {
                        line.to_string()
                    }
                }
            };
            stdout.println(format!(
                "\x1b[{}A\x1b[0J{}",
                previous_line_count, formatted_line
            ));
        } else {
            let formatted_line = match block_type {
                ContentBlock::UserInput => format!("  {}", line),
                _ => {
                    if is_bash_output {
                        format!("\x1b[90m{}\x1b[0m", line)
                    } else {
                        line.to_string()
                    }
                }
            };
            stdout.println(formatted_line);
        }
    }

    // Add empty line after each block for proper spacing, except for ToolStatus
    if !matches!(block_type, ContentBlock::ToolStatus) {
        stdout.println("");
        // Return total lines including the empty line
        wrapped_lines.len() + 1
    } else {
        // For ToolStatus, do not add a trailing empty line so that the following
        // ToolResult can visually attach to it as a single block
        wrapped_lines.len()
    }
}

/// Update status line at a specific position using ANSI escape sequences
///
/// # Parameters
/// - `stdout`: Output handler to write to
/// - `status_content`: Status line content to display
/// - `terminal_width`: Width for text wrapping
/// - `lines_from_bottom`: Number of lines from the bottom of the terminal
///
/// # Returns
/// The number of lines the status content occupies
pub fn update_status_line_at_position<T: OutputHandle>(
    stdout: &T,
    status_content: &str,
    terminal_width: usize,
    lines_from_bottom: usize,
) -> usize {
    let wrapped_lines = wrap_text(status_content, terminal_width);
    let status_line_count = wrapped_lines.len();

    // Save cursor position, move to status line position, clear and write, then restore
    for (i, line) in wrapped_lines.iter().enumerate() {
        if i == 0 {
            // Move to the status line position and clear the line
            stdout.println(format!(
                "\x1b[s\x1b[{}A\x1b[2K\r{}\x1b[u",
                lines_from_bottom, line
            ));
        } else {
            // For multi-line status, continue on next lines
            stdout.println(format!(
                "\x1b[s\x1b[{}A\x1b[2K\r{}\x1b[u",
                lines_from_bottom - i,
                line
            ));
        }
    }

    status_line_count
}

/// Apply ANSI color formatting to text
pub fn apply_color(text: &str, color: AnsiColor) -> String {
    match color {
        AnsiColor::Gray => format!("\x1b[90m{}\x1b[0m", text),
        AnsiColor::Green => format!("\x1b[32m{}\x1b[0m", text),
        AnsiColor::Yellow => format!("\x1b[33m{}\x1b[0m", text),
        AnsiColor::Blue => format!("\x1b[34m{}\x1b[0m", text),
        AnsiColor::Red => format!("\x1b[31m{}\x1b[0m", text),
        AnsiColor::Cyan => format!("\x1b[36m{}\x1b[0m", text),
        AnsiColor::White => format!("\x1b[37m{}\x1b[0m", text),
        AnsiColor::Reset => format!("\x1b[0m{}", text),
    }
}

/// ANSI color codes for terminal output
#[derive(Debug, Clone, Copy)]
pub enum AnsiColor {
    Gray,
    Green,
    Yellow,
    Blue,
    Red,
    Cyan,
    White,
    Reset,
}

/// Apply RGB color formatting to text
pub fn apply_rgb_color(text: &str, r: u8, g: u8, b: u8) -> String {
    format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock output handle for testing
    struct MockOutputHandle {
        pub output: std::sync::Mutex<Vec<String>>,
    }

    impl MockOutputHandle {
        fn new() -> Self {
            Self {
                output: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn get_output(&self) -> Vec<String> {
            self.output.lock().unwrap().clone()
        }
    }

    impl OutputHandle for MockOutputHandle {
        fn println<S: ToString>(&self, msg: S) {
            self.output.lock().unwrap().push(msg.to_string());
        }
    }

    #[test]
    fn test_output_content_block() {
        let mock = MockOutputHandle::new();
        let lines = output_content_block(&mock, "Test content", ContentBlock::AgentText, 80, false);

        let output = mock.get_output();
        assert!(!output.is_empty());
        assert!(lines > 0);
    }

    #[test]
    fn test_apply_color() {
        let colored = apply_color("test", AnsiColor::Red);
        assert!(colored.contains("\x1b[31m"));
        assert!(colored.contains("\x1b[0m"));
    }

    #[test]
    fn test_apply_rgb_color() {
        let colored = apply_rgb_color("test", 255, 0, 0);
        assert!(colored.contains("\x1b[38;2;255;0;0m"));
        assert!(colored.contains("\x1b[0m"));
    }
}
