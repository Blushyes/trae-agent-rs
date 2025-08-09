//! CLI-specific output implementations
//! 
//! This module contains concrete implementations of the output abstraction
//! for command-line interfaces, including formatting, colors, and interactive display.

pub mod cli_handler;
pub mod formatters;

pub use cli_handler::CliOutputHandler;
pub use formatters::*;
