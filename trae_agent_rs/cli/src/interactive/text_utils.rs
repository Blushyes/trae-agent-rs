//! Text processing utilities for interactive mode
//! 
//! This module provides text wrapping, Unicode-aware width calculation,
//! and other text processing utilities used by the interactive UI.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Wrap text to fit within specified width, breaking at word boundaries
/// Uses unicode-aware width calculation for proper handling of CJK characters
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
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

/// Get terminal width with fallback (used as fallback only)
pub fn get_terminal_width() -> usize {
    match crossterm::terminal::size() {
        Ok((cols, _)) => {
            // Reserve space for padding and borders, and ensure minimum width
            let usable_width = (cols as usize).saturating_sub(12); // 12 chars for padding/borders/safety
            std::cmp::max(usable_width, 30) // Minimum 30 chars
        }
        Err(_) => 68, // Fallback to 68 columns (80 - 12 for safety)
    }
}

/// Calculate the display width of text considering Unicode characters
pub fn text_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

/// Calculate the display width of a single character
pub fn char_width(ch: char) -> usize {
    UnicodeWidthChar::width(ch).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text_simple() {
        let text = "Hello world";
        let wrapped = wrap_text(text, 20);
        assert_eq!(wrapped, vec!["Hello world"]);
    }

    #[test]
    fn test_wrap_text_long_line() {
        let text = "This is a very long line that should be wrapped";
        let wrapped = wrap_text(text, 20);
        assert!(wrapped.len() > 1);
        for line in &wrapped {
            assert!(text_width(line) <= 20);
        }
    }

    #[test]
    fn test_wrap_text_unicode() {
        let text = "这是一个包含中文的测试文本";
        let wrapped = wrap_text(text, 10);
        for line in &wrapped {
            assert!(text_width(line) <= 10);
        }
    }

    #[test]
    fn test_terminal_width() {
        let width = get_terminal_width();
        assert!(width >= 30); // Should have minimum width
    }
}
