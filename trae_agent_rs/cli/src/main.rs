//! # Trae Agent CLI
//!
//! Command-line interface for Trae Agent - an LLM-based agent for software engineering tasks.
//!
//! This CLI provides both single-shot task execution and interactive modes,
//! with a beautiful terminal UI powered by iocraft.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;

mod commands;
mod interactive;
mod output;
mod ui;

use commands::{run_command, interactive_command, tools_command, test_command};

/// Trae Agent - LLM-based agent for software engineering tasks
#[derive(Parser)]
#[command(name = "trae")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "An LLM-based agent for software engineering tasks")]
#[command(long_about = None)]
struct Cli {
    /// Configuration directory path (for API provider configs)
    #[arg(short, long, default_value = ".")]
    config_dir: PathBuf,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Enable debug output mode (default is normal mode)
    #[arg(short = 'd', long = "debug")]
    debug_output: bool,

    /// Working directory
    #[arg(long)]
    working_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a single task
    Run {
        /// The task to execute
        task: String,

        /// Model provider to use
        #[arg(long)]
        provider: Option<String>,

        /// Model name to use
        #[arg(long)]
        model: Option<String>,

        /// API key for the model provider
        #[arg(long)]
        api_key: Option<String>,

        /// Maximum number of steps
        #[arg(long)]
        max_steps: Option<usize>,

        /// Output trajectory file
        #[arg(long)]
        trajectory_file: Option<PathBuf>,

        /// Must create a patch file
        #[arg(long)]
        must_patch: bool,

        /// Patch output file
        #[arg(long, default_value = "changes.patch")]
        patch_path: PathBuf,
    },

    /// Start interactive mode
    Interactive {
        /// Output trajectory file
        #[arg(long)]
        trajectory_file: Option<PathBuf>,

        /// Enable debug output mode
        #[arg(short = 'd', long = "debug")]
        debug_output: bool,
    },

    /// Show available tools
    Tools,

    /// Run basic tests
    Test,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose || cli.debug_output {
        "debug"
    } else {
        "info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
        .init();

    info!("Starting Trae Agent CLI v{}", env!("CARGO_PKG_VERSION"));

    // Change working directory if specified
    if let Some(working_dir) = &cli.working_dir {
        std::env::set_current_dir(working_dir)?;
        info!("Changed working directory to: {}", working_dir.display());
    }

    match cli.command {
        Some(Commands::Run {
            task,
            provider,
            model,
            api_key,
            max_steps,
            trajectory_file,
            must_patch,
            patch_path,
        }) => {
            run_command(
                task,
                cli.config_dir,
                provider,
                model,
                api_key,
                max_steps,
                trajectory_file,
                must_patch,
                patch_path,
                cli.working_dir,
                cli.debug_output,
            )
            .await
        }
        Some(Commands::Interactive {
            trajectory_file,
            debug_output,
        }) => {
            interactive_command(cli.config_dir, trajectory_file, debug_output).await
        }
        Some(Commands::Tools) => {
            tools_command().await
        }
        Some(Commands::Test) => {
            test_command().await
        }
        None => {
            // Default to interactive mode
            interactive_command(cli.config_dir, None, cli.debug_output).await
        }
    }
}
