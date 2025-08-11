//! TRAE logo component
//! 
//! This module provides the TRAE ASCII art logo component with gradient colors.

use iocraft::prelude::*;

// Static logo lines with individual colors for gradient effect
pub const TRAE_LOGO_LINES: &[&str] = &[
    " ███",
    "░░░███",
    "  ░░░███",
    "    ░░░███",
    "     ███░",
    "   ███░",
    " ███░",
    "░░░",
];

// Color gradient from bright green to darker green
pub const LOGO_COLORS: &[(u8, u8, u8)] = &[
    (0, 255, 127), // Bright green
    (0, 240, 120), // Slightly darker
    (0, 225, 113), // Medium bright
    (0, 210, 106), // Medium
    (0, 195, 99),  // Medium dark
    (0, 180, 92),  // Darker
    (0, 165, 85),  // Dark
    (0, 150, 78),  // Darkest
];

/// TRAE ASCII Art Logo Component with gradient colors
#[component]
pub fn TraeLogo(_hooks: Hooks) -> impl Into<AnyElement<'static>> {
    element! {
        View(key: "logo-content", flex_direction: FlexDirection::Column) {
            #(TRAE_LOGO_LINES.iter().enumerate().map(|(i, line)| {
                let color = LOGO_COLORS.get(i).unwrap_or(&(0, 150, 78));
                element! {
                    Text(
                        content: *line,
                        color: Color::Rgb { r: color.0, g: color.1, b: color.2 },
                        weight: Weight::Bold,
                    )
                }
            }).collect::<Vec<_>>())
        }
    }
}

/// Output TRAE logo to terminal with colors
pub fn output_logo_to_terminal<T: crate::interactive::terminal_output::OutputHandle>(stdout: &T) {
    // Output TRAE logo line by line with colors
    for (i, line) in TRAE_LOGO_LINES.iter().enumerate() {
        if !line.trim().is_empty() {
            let color = LOGO_COLORS.get(i).unwrap_or(&(0, 150, 78));
            // Use ANSI color codes for terminal output
            let colored_line = crate::interactive::terminal_output::apply_rgb_color(
                line, color.0, color.1, color.2
            );
            stdout.println(colored_line);
        }
    }
    stdout.println(""); // Empty line after logo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logo_lines_count() {
        assert_eq!(TRAE_LOGO_LINES.len(), 8);
        assert_eq!(LOGO_COLORS.len(), 8);
    }

    #[test]
    fn test_logo_colors_valid() {
        for color in LOGO_COLORS {
            assert!(color.0 <= 255);
            assert!(color.1 <= 255);
            assert!(color.2 <= 255);
        }
    }

    #[test]
    fn test_logo_lines_not_empty() {
        for line in TRAE_LOGO_LINES {
            assert!(!line.is_empty());
        }
    }
}
